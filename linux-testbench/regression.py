import json
from pathlib import Path


def reports(directory):
    directory = Path(directory)
    result = {}
    for path in sorted(
        [
            *directory.glob("auto/*/report.json"),
            *directory.glob("vm/*/auto/*/report.json"),
        ]
    ):
        report = json.loads(path.read_text())
        configuration = path.parent / "configuration.json"
        if configuration.exists():
            report["configuration"] = json.loads(configuration.read_text())
        result[str(path.relative_to(directory))] = report
    return result


def compare(baseline, current):
    regressions = []
    improvements = []
    for profile, previous in baseline.items():
        latest = {test["name"]: test for test in current.get(profile, {}).get("tests", [])}
        for test in previous["tests"]:
            actual = latest.get(test["name"])
            if test["status"] == "passed" and (actual is None or actual["status"] != "passed"):
                regressions.append(
                    {
                        "profile": profile,
                        "test": test["name"],
                        "status": actual["status"] if actual else "missing",
                    }
                )
            elif test["status"] != "passed" and actual and actual["status"] == "passed":
                improvements.append({"profile": profile, "test": test["name"]})
    for profile, report in current.items():
        if report.get("selection") is not None:
            regressions.append(
                {"profile": profile, "test": "coverage.must-be-complete", "status": "partial"}
            )
        previous_names = {test["name"] for test in baseline.get(profile, {}).get("tests", [])}
        for test in report["tests"]:
            if (
                test["required"]
                and test["status"] != "passed"
                and test["name"] not in previous_names
            ):
                regressions.append(
                    {"profile": profile, "test": test["name"], "status": test["status"]}
                )
    return {"regressions": regressions, "improvements": improvements}


def snapshot(directory):
    path = Path(directory) / "baseline.json"
    if path.exists():
        raise ValueError("Baseline already exists; preserve it until the comparison is complete")
    path.write_text(json.dumps({"schema": 1, "reports": reports(directory)}, indent=2) + "\n")
    print("Baseline: " + str(path))


def check(directory):
    directory = Path(directory)
    baseline = json.loads((directory / "baseline.json").read_text())["reports"]
    current = reports(directory)
    result = compare(baseline, current)
    import hashlib

    import lab

    digests = {}
    for profile, report in current.items():
        configuration = report.get("configuration")
        if not configuration:
            continue
        kind = configuration["profile"]["artifact"]
        if kind not in digests:
            digests[kind] = hashlib.sha256(lab.release(kind).read_bytes()).hexdigest()
        if configuration["artifactSha256"] != digests[kind]:
            result["regressions"].append(
                {"profile": profile, "test": "artifact.must-match-current-build", "status": "stale"}
            )
    (directory / "comparison.json").write_text(json.dumps(result, indent=2) + "\n")
    for kind, items in result.items():
        print(f"{kind}: {len(items)}")
        for item in items:
            print(
                f"  {item['profile']}: {item['test']}"
                + (" (" + item["status"] + ")" if "status" in item else "")
            )
    if result["regressions"]:
        raise SystemExit(1)
