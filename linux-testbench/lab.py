import argparse
import hashlib
import html
import json
import os
import re
import shutil
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
PROJECT = HERE.parent
DATA = PROJECT / "build/linux-testbench"
CACHE = PROJECT / "build/.cache/linux-testbench"
CATALOG = json.loads((HERE / "presets.json").read_text())
LABEL = "dev.simplevoiceover.linux-testbench=true"


def user_namespace():
    return (
        "--userns=keep-id"
        if os.getuid() == 1000 and os.getgid() == 1000
        else "--userns=keep-id:uid=1000,gid=1000"
    )


def run(arguments, **options):
    return subprocess.run([str(argument) for argument in arguments], check=True, **options)


def output(arguments):
    return subprocess.check_output([str(argument) for argument in arguments], text=True).strip()


def profile(name, arguments):
    if name in CATALOG["presets"]:
        value = dict(CATALOG["presets"][name])
    else:
        path = Path(name)
        custom = json.loads(path.read_text())
        value = dict(CATALOG["presets"][custom.pop("extends")])
        value.update(custom)
        name = path.stem
    for key in ["audio", "artifact", "codecs", "display", "webkit", "webkit_version", "base_image"]:
        override = getattr(arguments, key, None)
        if override is not None:
            value[key] = override
    value.setdefault("display", "x11")
    value.setdefault("webkit_version", "")
    value.setdefault("webkit", "bundled" if value["artifact"] == "appimage" else "system")
    distribution = CATALOG["distributions"][value["distribution"]]
    value["base_image"] = value.get("base_image", distribution["image"])
    value["family"] = distribution["family"]
    for key, choices in [
        ("audio", ["pulseaudio", "pipewire"]),
        ("artifact", ["deb", "rpm", "appimage"]),
        ("codecs", ["minimal", "full"]),
        ("display", ["x11", "wayland"]),
        ("webkit", ["system", "bundled"]),
    ]:
        if value[key] not in choices:
            raise ValueError(f"Invalid {key}: {value[key]}")
    if value["artifact"] in ["deb", "rpm"] and value["artifact"] != value["family"]:
        raise ValueError(
            "Native package format must match the distribution; use AppImage for cross-distribution checks"
        )
    if value["artifact"] != "appimage" and value["webkit"] != "system":
        raise ValueError("Bundled WebKit is available only with AppImage")
    if value["webkit"] == "bundled" and value["webkit_version"]:
        raise ValueError(
            "AppImage contains its own WebKit version; use --webkit system to test distribution package versions"
        )
    if not re.fullmatch(r"[A-Za-z0-9_.-]+", name):
        raise ValueError("Preset name must contain only letters, numbers, '.', '_' and '-'")
    value["name"] = name
    return value


def image_name(value):
    tag = "-".join([value["distribution"], value["audio"], value["codecs"]])
    default_image = CATALOG["distributions"][value["distribution"]]["image"]
    if value["webkit_version"] or value["base_image"] != default_image:
        digest = hashlib.sha256(
            json.dumps([value["webkit_version"], value["base_image"]]).encode()
        ).hexdigest()[:12]
        tag += "-" + digest
    return "localhost/svoice-linux-testbench:" + tag


def image_exists(name):
    return (
        subprocess.run(
            ["podman", "image", "exists", name],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        ).returncode
        == 0
    )


def prepare(value, refresh=False):
    image = image_name(value)
    if image_exists(image) and not refresh:
        print("Ready: " + image, flush=True)
        return
    logs = DATA / "setup"
    logs.mkdir(parents=True, exist_ok=True)
    arguments = [
        "podman",
        "build",
        "--label",
        LABEL,
        "--tag",
        image,
        "--file",
        HERE / "Containerfile",
        "--build-arg",
        "BASE_IMAGE=" + value["base_image"],
        "--build-arg",
        "AUDIO=" + value["audio"],
        "--build-arg",
        "CODECS=" + value["codecs"],
        "--build-arg",
        "WEBKIT_VERSION=" + value["webkit_version"],
    ]
    if refresh:
        arguments.extend(["--pull=always", "--no-cache"])
    arguments.append(HERE)
    path = logs / (image.split(":")[-1] + ".log")
    print(f"Preparing {image}; log: {path}", flush=True)
    with path.open("w") as log:
        try:
            run(arguments, stdout=log, stderr=subprocess.STDOUT)
        except subprocess.CalledProcessError:
            print("\n".join(path.read_text(errors="replace").splitlines()[-30:]), file=sys.stderr)
            raise
    inventory = json.loads(output(["podman", "image", "inspect", image]))[0]
    (logs / (image.split(":")[-1] + ".json")).write_text(
        json.dumps(
            {
                "profile": value,
                "imageId": inventory["Id"],
                "baseImage": value["base_image"],
                "created": inventory["Created"],
            },
            indent=2,
        )
        + "\n"
    )
    print("Ready: " + image, flush=True)


def fixtures(refresh=False):
    path = DATA / "fixtures"
    manifest = path / "manifest.json"
    if manifest.exists() and not refresh:
        value = json.loads(manifest.read_text())
        if all(
            (path / item["file"]).exists()
            and hashlib.sha256((path / item["file"]).read_bytes()).hexdigest() == item["sha256"]
            for item in value["files"]
        ):
            print("Procedural fixtures ready: " + str(path), flush=True)
            return
    value = profile("ubuntu22-pulse", argparse.Namespace())
    prepare(value)
    path.mkdir(parents=True, exist_ok=True)
    run(
        [
            "podman",
            "run",
            "--rm",
            "--label",
            LABEL,
            "--security-opt=label=disable",
            user_namespace(),
            "--user=1000:1000",
            "--volume",
            f"{path}:/fixtures",
            "--volume",
            f"{HERE}:/opt/linux-testbench:ro",
            "--entrypoint=python3",
            image_name(value),
            "-B",
            "/opt/linux-testbench/fixtures.py",
            "/fixtures",
        ]
    )


def bundled_driver():
    value = profile("ubuntu22-pulse", argparse.Namespace())
    prepare(value)
    image_id = json.loads(output(["podman", "image", "inspect", image_name(value)]))[0]["Id"]
    directory = CACHE / "webdriver"
    marker = directory / "image-id"
    if marker.exists() and marker.read_text() == image_id:
        return directory
    if directory.exists():
        shutil.rmtree(directory)
    directory.mkdir(parents=True)
    run(
        [
            "podman",
            "run",
            "--rm",
            "--label",
            LABEL,
            "--security-opt=label=disable",
            user_namespace(),
            "--user=1000:1000",
            "--volume",
            f"{directory}:/tools",
            "--volume",
            f"{HERE}:/opt/linux-testbench:ro",
            "--entrypoint=python3",
            image_name(value),
            "-B",
            "/opt/linux-testbench/driver.py",
            "/tools",
        ]
    )
    marker.write_text(image_id)
    return directory


def release(kind):
    version = json.loads((PROJECT / "package.json").read_text())["version"]
    pattern = {
        "deb": f"simpleVoiceover_{version}_amd64.deb",
        "rpm": f"simpleVoiceover-{version}-*.x86_64.rpm",
        "appimage": f"simpleVoiceover_{version}_amd64.AppImage",
    }[kind]
    candidates = list((PROJECT / "build").glob(pattern))
    if len(candidates) != 1:
        raise ValueError(
            f"Expected one {kind} release for version {version} in build/; found {len(candidates)}"
        )
    return candidates[0]


def instance(value, mode):
    configuration = {
        key: value[key]
        for key in [
            "distribution",
            "audio",
            "artifact",
            "codecs",
            "display",
            "webkit",
            "webkit_version",
            "base_image",
        ]
    }
    digest = hashlib.sha256(json.dumps(configuration, sort_keys=True).encode()).hexdigest()[:8]
    return "svoice-lab-" + mode + "-" + value["name"] + "-" + digest


def session_environment(name, results):
    environment = json.loads((results / "session-environment.json").read_text())
    arguments = ["podman", "exec", "--user=1000:1000"]
    for key, value in environment.items():
        arguments.extend(["--env", key + "=" + value])
    return arguments + [name]


def start(value, manual=False, bridge="virtual", port=6080):
    prepare(value)
    fixtures()
    mode = "manual" if manual else "auto"
    name = instance(value, mode)
    results = DATA / mode / name
    if (
        subprocess.run(
            ["podman", "container", "exists", name], stdout=subprocess.DEVNULL, check=False
        ).returncode
        == 0
    ):
        raise ValueError(f"{name} already exists. Stop it with: linux-testbench/lab stop {name}")
    if not manual and results.exists():
        shutil.rmtree(results)
    results.mkdir(parents=True, exist_ok=True)
    (results / "ready").unlink(missing_ok=True)
    artifact = release(value["artifact"])
    suffix = {"deb": "deb", "rpm": "rpm", "appimage": "AppImage"}[value["artifact"]]
    arguments = [
        "podman",
        "run",
        "--detach",
        "--name",
        name,
        "--label",
        LABEL,
        "--security-opt=label=disable",
        user_namespace(),
        "--user=0:0",
        "--shm-size=1g",
        "--volume",
        f"{HERE}:/opt/linux-testbench:ro",
        "--volume",
        f"{artifact}:/artifact/release.{suffix}:ro",
        "--volume",
        f"{DATA / 'fixtures'}:/fixtures:ro",
        "--volume",
        f"{results}:/results",
    ]
    if value["webkit"] == "bundled":
        arguments.extend(["--volume", f"{bundled_driver()}:/tools/webdriver:ro"])
    if manual:
        arguments.extend(["--publish", f"127.0.0.1:{port}:6080"])
    if bridge == "host":
        runtime = Path(os.environ.get("XDG_RUNTIME_DIR", f"/run/user/{os.getuid()}"))
        socket = runtime / "pulse/native"
        if not socket.exists():
            raise ValueError(
                "Host PulseAudio-compatible socket is unavailable; select --bridge virtual"
            )
        arguments.extend(["--volume", f"{socket}:/run/studio-host/pulse-native:ro"])
        cookie = Path.home() / ".config/pulse/cookie"
        if cookie.exists():
            arguments.extend(
                [
                    "--volume",
                    f"{cookie}:/run/studio-host/pulse-cookie:ro",
                    "--env",
                    "PULSE_COOKIE=/run/studio-host/pulse-cookie",
                ]
            )
    for key, item in {
        "LAB_AUDIO": value["audio"],
        "LAB_PROFILE": name,
        "LAB_ARTIFACT": value["artifact"],
        "LAB_WEBKIT": value["webkit"],
        "LAB_CODECS": value["codecs"],
        "LAB_DISPLAY": value["display"],
        "LAB_BRIDGE": bridge,
        "LAB_MANUAL": "1" if manual else "0",
    }.items():
        arguments.extend(["--env", key + "=" + item])
    arguments.append(image_name(value))
    print("Starting " + name, flush=True)
    run(arguments, stdout=subprocess.DEVNULL)
    source_digest = hashlib.sha256()
    for source in sorted(HERE.glob("*")):
        if source.is_file():
            source_digest.update(source.name.encode() + b"\0" + source.read_bytes())
    metadata = {
        "profile": value,
        "container": name,
        "artifact": artifact.name,
        "artifactSha256": hashlib.sha256(artifact.read_bytes()).hexdigest(),
        "image": json.loads(output(["podman", "image", "inspect", image_name(value)]))[0]["Id"],
        "kernelScope": "host-shared",
        "bridge": bridge,
        "harnessSha256": source_digest.hexdigest(),
        "startedAt": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
    }
    if value["webkit"] == "bundled":
        metadata["automationDriver"] = json.loads((CACHE / "webdriver/driver.json").read_text())
    (results / "configuration.json").write_text(json.dumps(metadata, indent=2) + "\n")
    deadline = time.monotonic() + 180
    while not (results / "ready").exists():
        state = output(["podman", "inspect", "--format", "{{.State.Status}}", name])
        if state != "running" or time.monotonic() > deadline:
            logs = subprocess.check_output(
                ["podman", "logs", name], text=True, stderr=subprocess.STDOUT
            )
            (results / "container.log").write_text(logs)
            failure = {
                "name": "package.install-or-desktop.setup",
                "status": "failed",
                "required": True,
                "seconds": 0,
                "error": logs[-12000:],
            }
            (results / "report.json").write_text(
                json.dumps(
                    {"schema": 1, "preset": name, "environment": metadata, "tests": [failure]},
                    indent=2,
                )
                + "\n"
            )
            import xml.etree.ElementTree as ET

            suite = ET.Element("testsuite", name=name, tests="1", failures="1")
            ET.SubElement(
                ET.SubElement(suite, "testcase", name=failure["name"]),
                "failure",
                message="Package installation or desktop setup failed",
            ).text = failure["error"]
            ET.ElementTree(suite).write(
                results / "junit.xml", encoding="utf-8", xml_declaration=True
            )
            print("\n".join(logs.splitlines()[-30:]), file=sys.stderr)
            raise RuntimeError("Test bench failed to start; inspect " + str(results))
        time.sleep(0.5)
    return name, results


def stop(name):
    inventory = json.loads(output(["podman", "inspect", name]))[0]
    if (
        inventory.get("Config", {}).get("Labels", {}).get("dev.simplevoiceover.linux-testbench")
        != "true"
    ):
        raise ValueError("Refusing to stop a container outside this test bench")
    run(["podman", "stop", "--time=5", name], stdout=subprocess.DEVNULL)
    run(["podman", "rm", name], stdout=subprocess.DEVNULL)
    print("Stopped " + name, flush=True)


def containers():
    return output(
        ["podman", "ps", "--all", "--filter", "label=" + LABEL, "--format", "{{.Names}}"]
    ).splitlines()


def auto(values, extended=False, keep=False, match=None):
    failed = False
    for value in values:
        name = None
        existing = set(containers())
        try:
            name, results = start(value)
            arguments = session_environment(name, results) + [
                "python3",
                "-B",
                "/opt/linux-testbench/checks.py",
            ]
            if extended:
                arguments.append("--extended")
            if match:
                arguments.extend(["--match", match])
            run(arguments)
        except (subprocess.CalledProcessError, RuntimeError, ValueError) as error:
            failed = True
            print(str(error), file=sys.stderr)
        finally:
            if name is None:
                name = instance(value, "auto")
            if not keep and name not in existing and name in containers():
                stop(name)
    report_index()
    if failed:
        raise SystemExit(1)


def report_index():
    reports = []
    for path in sorted([*DATA.glob("auto/*/report.json"), *DATA.glob("vm/*/auto/*/report.json")]):
        value = json.loads(path.read_text())
        tests = value.get("tests", [])
        reports.append((path, value, tests))
    rows = []
    for path, value, tests in reports:
        failed = sum(test["status"] == "failed" for test in tests)
        passed = sum(test["status"] == "passed" for test in tests)
        skipped = sum(test["status"] == "skipped" for test in tests)
        link = str(path.relative_to(DATA))
        scope = path.parts[-4] if "vm" in path.relative_to(DATA).parts else "host kernel"
        rows.append(
            f'<tr><td>{html.escape(value.get("preset", path.parent.name))}<br><small>{html.escape(scope)}</small></td><td>{passed}</td><td class="{"bad" if failed else "good"}">{failed}</td><td>{skipped}</td><td><a href="{html.escape(link)}">JSON</a> · <a href="{html.escape(link.replace("report.json", "junit.xml"))}">JUnit</a></td></tr>'
        )
    DATA.mkdir(parents=True, exist_ok=True)
    document = (
        '<!doctype html><html lang="en"><meta charset="utf-8"><title>Linux compatibility test bench</title><style>body{font:16px system-ui;background:#10151d;color:#e7edf7;margin:3rem;max-width:1100px}table{border-collapse:collapse;width:100%}td,th{text-align:left;padding:12px;border-bottom:1px solid #394356}a{color:#9fc7ff}.bad{color:#ff9898}.good{color:#a5e5b1}</style><h1>Linux compatibility test bench</h1><p>Procedural media only. Container kernels belong to the host. Guest kernels are recorded separately by VM runs.</p><table><tr><th>Preset</th><th>Passed</th><th>Failed</th><th>Skipped</th><th>Evidence</th></tr>'
        + "".join(rows)
        + "</table></html>"
    )
    (DATA / "index.html").write_text(document)
    print("Reports: " + str(DATA / "index.html"), flush=True)


def doctor():
    for name in [
        "podman",
        "python3",
        "qemu-system-x86_64",
        "qemu-img",
        "genisoimage",
        "ssh",
        "ssh-keygen",
    ]:
        print(f"{name:24} {shutil.which(name) or 'missing'}")
    print(
        f"{'KVM':24} {'available' if os.access('/dev/kvm', os.R_OK | os.W_OK) else 'unavailable'}"
    )
    for kind in ["deb", "rpm", "appimage"]:
        try:
            print(f"{kind:24} {release(kind)}")
        except ValueError as error:
            print(str(error))


def parser():
    root = argparse.ArgumentParser(
        description="Linux compatibility test bench: presets plus independent overrides"
    )
    commands = root.add_subparsers(dest="command", required=True)
    commands.add_parser("list", help="List distribution/audio presets and VM kernel profiles")
    commands.add_parser("doctor", help="Check local tools and release artifacts")
    media = commands.add_parser(
        "fixtures", help="Generate original procedural audio/video fixtures"
    )
    media.add_argument("--refresh", action="store_true")
    for command in ["prepare", "auto", "manual"]:
        child = commands.add_parser(command)
        child.add_argument(
            "presets", nargs="*", help="Built-in names or JSON files extending a preset"
        )
        child.add_argument("--all", action="store_true")
        child.add_argument("--audio", choices=["pulseaudio", "pipewire"])
        child.add_argument("--artifact", choices=["deb", "rpm", "appimage"])
        child.add_argument("--codecs", choices=["minimal", "full"])
        child.add_argument("--display", choices=["x11", "wayland"])
        child.add_argument(
            "--webkit",
            choices=["system", "bundled"],
            help="AppImage may also run its extracted ELF against system libraries; this does not test its launcher",
        )
        child.add_argument("--webkit-version")
        child.add_argument(
            "--base-image",
            help="Override the distribution base with an OCI digest or repository snapshot image",
        )
        if command == "prepare":
            child.add_argument("--refresh", action="store_true")
        elif command == "auto":
            child.add_argument("--extended", action="store_true")
            child.add_argument(
                "--match",
                help="Run only cases matching this regular expression, plus desktop startup",
            )
            child.add_argument(
                "--keep", action="store_true", help="Keep containers after checks for inspection"
            )
        else:
            child.add_argument("--bridge", choices=["host", "virtual"], default="host")
            child.add_argument("--port", type=int, default=6080)
    child = commands.add_parser("stop")
    child.add_argument(
        "name", nargs="?", help="Container name; omit to stop all test-bench containers"
    )
    child = commands.add_parser("shell")
    child.add_argument("name")
    child = commands.add_parser("audio")
    child.add_argument("name")
    child.add_argument("action", choices=["devices", "default", "remove", "restore"])
    child.add_argument("device", nargs="?", choices=["a", "b"])
    commands.add_parser("report")
    commands.add_parser(
        "baseline", help="Preserve current JSON reports before changing the application"
    )
    commands.add_parser(
        "compare", help="Fail if a previously passing check is missing or no longer passes"
    )
    commands.add_parser(
        "clean",
        help="Remove only this bench's containers, images, generated fixtures, reports and VM disks",
    )
    child = commands.add_parser("vm")
    child.add_argument("action", choices=["prepare", "manual", "auto", "stop"])
    child.add_argument("name", choices=list(CATALOG["virtual_machines"]))
    child.add_argument("--extended", action="store_true")
    child.add_argument("--preset", help="Use any userspace preset on the selected guest kernel")
    return root


def main():
    arguments = parser().parse_args()
    if arguments.command == "list":
        print(f"{'PRESET':24} {'DISTRIBUTION':20} {'AUDIO':14} {'ARTIFACT':10} WEBKIT")
        for name, value in CATALOG["presets"].items():
            distro = CATALOG["distributions"][value["distribution"]]
            print(
                f"{name:24} {distro['label']:20} {value['audio']:14} {value['artifact']:10} {value.get('webkit', 'bundled' if value['artifact'] == 'appimage' else 'system 4.1')}"
            )
        print("\nVM kernel profiles:")
        for name, value in CATALOG["virtual_machines"].items():
            print(f"{name:24} {value['kernel_description']}")
    elif arguments.command == "doctor":
        doctor()
    elif arguments.command == "fixtures":
        fixtures(arguments.refresh)
    elif arguments.command in ["prepare", "auto", "manual"]:
        names = (
            list(CATALOG["presets"]) if arguments.all else arguments.presets or ["ubuntu22-pulse"]
        )
        values = [profile(name, arguments) for name in names]
        if arguments.command == "prepare":
            for value in values:
                prepare(value, arguments.refresh)
        elif arguments.command == "auto":
            auto(values, arguments.extended, arguments.keep, arguments.match)
        else:
            if len(values) != 1:
                raise ValueError(
                    "Manual mode starts one preset at a time; use different ports for simultaneous sessions"
                )
            name, results = start(
                values[0], manual=True, bridge=arguments.bridge, port=arguments.port
            )
            print(
                f"Desktop: http://127.0.0.1:{arguments.port}/vnc.html?autoconnect=1&resize=scale\nContainer: {name}\nWorkspace: {results}\nStop: linux-testbench/lab stop {name}"
            )
    elif arguments.command == "stop":
        for name in [arguments.name] if arguments.name else containers():
            stop(name)
    elif arguments.command in ["baseline", "compare"]:
        import regression

        (regression.snapshot if arguments.command == "baseline" else regression.check)(DATA)
    elif arguments.command in ["shell", "audio"]:
        if arguments.name not in containers():
            raise ValueError("No such test-bench container")
        inventory = json.loads(output(["podman", "inspect", arguments.name]))[0]
        results = Path(
            next(
                mount["Source"]
                for mount in inventory["Mounts"]
                if mount["Destination"] == "/results"
            )
        )
        prefix = session_environment(arguments.name, results)
        if arguments.command == "shell":
            prefix.insert(2, "--interactive")
            prefix.insert(3, "--tty")
            run(prefix + ["bash", "--rcfile", "/opt/linux-testbench/interactive.bash"])
        else:
            if arguments.action != "devices" and not arguments.device:
                raise ValueError("Select device a or b")
            run(
                prefix
                + ["python3", "-B", "/opt/linux-testbench/audio.py", arguments.action]
                + ([arguments.device] if arguments.device else [])
            )
    elif arguments.command == "report":
        report_index()
    elif arguments.command == "vm":
        import vm

        vm.command(arguments, CATALOG, HERE, DATA, CACHE)
    elif arguments.command == "clean":
        import vm

        for name in CATALOG["virtual_machines"]:
            vm.stop(CACHE / "vm" / name)
        for name in containers():
            stop(name)
        images = output(
            ["podman", "images", "--filter", "label=" + LABEL, "--format", "{{.ID}}"]
        ).splitlines()
        if images:
            run(["podman", "rmi", *sorted(set(images))])
        for path in [DATA, CACHE]:
            if path.exists():
                shutil.rmtree(path)
        print(
            "Removed this test bench's generated data. Source files and application releases remain available."
        )


if __name__ == "__main__":
    try:
        main()
    except (ValueError, RuntimeError, subprocess.CalledProcessError) as error:
        print(str(error), file=sys.stderr)
        raise SystemExit(1)
