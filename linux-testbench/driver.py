import hashlib
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path


def extract(destination):
    destination.mkdir(parents=True, exist_ok=True)
    libraries = destination / "lib"
    libraries.mkdir(exist_ok=True)
    executable = Path(shutil.which("WebKitWebDriver"))
    linkage = subprocess.check_output(["ldd", str(executable)], text=True)
    if "not found" in linkage:
        raise RuntimeError(linkage)
    for source in set(re.findall(r"(/[^\s()]+)", linkage)):
        path = Path(source)
        target = (
            destination / path.name
            if path.name == "ld-linux-x86-64.so.2"
            else libraries / path.name
        )
        shutil.copyfile(path, target)
        target.chmod(0o755)
    shutil.copyfile(executable, destination / executable.name)
    (destination / executable.name).chmod(0o755)
    version = subprocess.check_output(
        ["dpkg-query", "-W", "--showformat=${Version}", "webkit2gtk-driver"], text=True
    )
    metadata = {
        "version": version,
        "source": "Ubuntu 22.04 webkit2gtk-driver",
        "linkage": linkage,
        "sha256": hashlib.sha256(executable.read_bytes()).hexdigest(),
    }
    (destination / "driver.json").write_text(json.dumps(metadata, indent=2) + "\n")


if __name__ == "__main__":
    extract(Path(sys.argv[1]))
