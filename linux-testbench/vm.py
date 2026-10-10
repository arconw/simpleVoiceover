import hashlib
import json
import os
import shlex
import signal
import socket
import subprocess
import time
import urllib.parse
import urllib.request
from pathlib import Path


def run(arguments, **options):
    return subprocess.run([str(argument) for argument in arguments], check=True, **options)


def download(url, destination):
    temporary = destination.with_suffix(destination.suffix + ".partial")
    run(["curl", "--fail", "--location", "--retry=3", "--output", temporary, url])
    temporary.replace(destination)


def free_port():
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        return listener.getsockname()[1]


def prepare(name, config, cache):
    directory = cache / "vm" / name
    directory.mkdir(parents=True, exist_ok=True)
    key = cache / "vm" / "ssh-key"
    if not key.exists():
        run(["ssh-keygen", "-q", "-t", "ed25519", "-N", "", "-f", key])
    source_name = Path(urllib.parse.urlparse(config["image"]).path).name
    bases = cache / "vm" / "bases"
    bases.mkdir(exist_ok=True)
    base = bases / source_name
    if not base.exists():
        print("Downloading shared VM base: " + config["image"], flush=True)
        with urllib.request.urlopen(config["checksums"], timeout=30) as response:
            checksums = response.read().decode()
        digest = next(
            line.split()[0]
            for line in checksums.splitlines()
            if line.split()[-1].lstrip("*") == source_name
        )
        download(config["image"], base)
        with base.open("rb") as handle:
            actual = hashlib.file_digest(handle, "sha256").hexdigest()
        if actual != digest:
            base.unlink()
            raise ValueError("Cloud image SHA256 mismatch; base was removed")
        (bases / (source_name + ".json")).write_text(
            json.dumps(
                {"url": config["image"], "sha256": actual, "checksumSource": config["checksums"]},
                indent=2,
            )
            + "\n"
        )
    disk = directory / "disk.qcow2"
    if not disk.exists():
        run(["qemu-img", "create", "-f", "qcow2", "-F", "qcow2", "-b", base, disk])
        run(["qemu-img", "resize", disk, "32G"])
    seed = directory / "seed"
    seed.mkdir(exist_ok=True)
    user = {
        "name": "studio",
        "uid": 1000,
        "shell": "/bin/bash",
        "groups": ["sudo", "audio"],
        "sudo": ["ALL=(ALL) NOPASSWD:ALL"],
        "lock_passwd": True,
        "ssh_authorized_keys": [key.with_suffix(".pub").read_text().strip()],
    }
    user_data = {
        "users": [user],
        "ssh_pwauth": False,
        "package_update": True,
        "packages": [
            "podman",
            "python3",
            "dbus-user-session",
            "pulseaudio",
            "pulseaudio-utils",
            "alsa-utils",
            config["kernel_package"],
        ],
        "runcmd": [
            ["loginctl", "enable-linger", "studio"],
            ["mkdir", "-p", "/var/lib/studio-lab"],
            ["touch", "/var/lib/studio-lab/provisioned"],
        ],
        "power_state": {
            "mode": "reboot",
            "delay": "now",
            "message": "Boot the selected test-bench kernel",
            "condition": True,
        },
    }
    (seed / "user-data").write_text("#cloud-config\n" + json.dumps(user_data, indent=2) + "\n")
    (seed / "meta-data").write_text(
        json.dumps({"instance-id": "simplevoiceover-" + name, "local-hostname": "svoice-" + name})
        + "\n"
    )
    run(
        [
            "genisoimage",
            "-quiet",
            "-output",
            directory / "seed.iso",
            "-volid",
            "cidata",
            "-joliet",
            "-rock",
            seed / "user-data",
            seed / "meta-data",
        ]
    )
    (directory / "profile.json").write_text(json.dumps(config, indent=2) + "\n")
    print("VM disk ready: " + str(directory), flush=True)
    return directory, key


def alive(directory, filename="qemu.pid"):
    pid_file = directory / filename
    if not pid_file.exists():
        return False
    try:
        pid = int(pid_file.read_text())
        command = Path(f"/proc/{pid}/cmdline").read_bytes().replace(b"\0", b" ").decode()
        expected = str(
            directory
            / {
                "qemu.pid": "disk.qcow2",
                "forward.pid": "known-hosts",
                "host-audio.pid": "host-audio.log",
            }[filename]
        )
        return expected in command
    except (OSError, ValueError):
        return False


def ssh_options(directory, key, port):
    return [
        "-i",
        str(key),
        "-o",
        "BatchMode=yes",
        "-o",
        "ConnectTimeout=5",
        "-o",
        "StrictHostKeyChecking=accept-new",
        "-o",
        "UserKnownHostsFile=" + str(directory / "known-hosts"),
        "-p",
        str(port),
    ]


def ssh(directory, key, port, command, **options):
    return run(["ssh", *ssh_options(directory, key, port), "studio@127.0.0.1", command], **options)


def start(directory, key, manual):
    if alive(directory):
        state = json.loads((directory / "state.json").read_text())
        if state["hostAudio"] != manual:
            stop(directory)
        else:
            return state["sshPort"]
    if not os.access("/dev/kvm", os.R_OK | os.W_OK):
        raise ValueError("KVM is unavailable; these VM profiles require hardware virtualization")
    port = free_port()
    arguments = [
        "qemu-system-x86_64",
        "-name",
        "svoice-testbench-" + directory.name,
        "-machine",
        "q35,accel=kvm",
        "-cpu",
        "host",
        "-smp",
        "4",
        "-m",
        "6144",
        "-drive",
        f"file={directory / 'disk.qcow2'},if=virtio,format=qcow2",
        "-drive",
        f"file={directory / 'seed.iso'},media=cdrom,readonly=on",
        "-device",
        "virtio-net-pci,netdev=net0",
        "-netdev",
        f"user,id=net0,hostfwd=tcp:127.0.0.1:{port}-:22",
        "-display",
        "none",
        "-serial",
        "file:" + str(directory / "console.log"),
        "-qmp",
        "unix:" + str(directory / "qmp.sock") + ",server=on,wait=off",
        "-pidfile",
        directory / "qemu.pid",
        "-daemonize",
        "-audiodev",
        ("pa" if manual else "none") + ",id=sound0",
        "-device",
        "intel-hda",
        "-device",
        "hda-duplex,audiodev=sound0",
    ]
    identifier = "dev.simplevoiceover.linux-testbench.vm." + directory.name
    run(arguments, env=dict(os.environ, PULSE_PROP="application.id=" + identifier))
    if manual:
        runtime = Path(os.environ.get("XDG_RUNTIME_DIR", f"/run/user/{os.getuid()}"))
        follower = subprocess.Popen(
            [
                "python3",
                "-B",
                str(Path(__file__).parent / "audio.py"),
                "follow",
                identifier,
                str(directory / "host-audio.log"),
            ],
            env=dict(
                os.environ,
                LAB_HOST_SERVER=os.environ.get(
                    "PULSE_SERVER", "unix:" + str(runtime / "pulse/native")
                ),
            ),
            start_new_session=True,
        )
        (directory / "host-audio.pid").write_text(str(follower.pid))
    (directory / "state.json").write_text(
        json.dumps({"sshPort": port, "hostAudio": manual}, indent=2) + "\n"
    )
    print(f"Booting {directory.name}; cloud-init progress: {directory / 'console.log'}", flush=True)
    deadline = time.monotonic() + 1200
    announced = 0
    while time.monotonic() < deadline:
        if not alive(directory):
            raise RuntimeError("VM exited; inspect " + str(directory / "console.log"))
        probe = subprocess.run(
            [
                "ssh",
                *ssh_options(directory, key, port),
                "studio@127.0.0.1",
                "test -f /var/lib/studio-lab/provisioned && test ! -f /run/reboot-required && uname -r",
            ],
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            check=False,
        )
        if probe.returncode == 0:
            time.sleep(8)
            try:
                kernel = (
                    ssh(
                        directory,
                        key,
                        port,
                        "cloud-init status --wait >/dev/null; uname -r",
                        stdout=subprocess.PIPE,
                        stderr=subprocess.DEVNULL,
                    )
                    .stdout.decode()
                    .strip()
                )
                print("Guest kernel: " + kernel, flush=True)
                (directory / "kernel.txt").write_text(kernel + "\n")
                return port
            except subprocess.CalledProcessError:
                pass
        if time.monotonic() - announced >= 30:
            print("Waiting for VM provisioning and kernel reboot...", flush=True)
            announced = time.monotonic()
        time.sleep(3)
    raise RuntimeError("VM provisioning timed out; inspect console.log")


def stop(directory):
    if alive(directory, "host-audio.pid"):
        os.kill(int((directory / "host-audio.pid").read_text()), signal.SIGTERM)
    if alive(directory, "forward.pid"):
        os.kill(int((directory / "forward.pid").read_text()), signal.SIGTERM)
    if alive(directory):
        try:
            with socket.socket(socket.AF_UNIX) as connection:
                connection.settimeout(3)
                connection.connect(str(directory / "qmp.sock"))
                stream = connection.makefile("rwb")
                stream.readline()
                stream.write(b'{"execute":"qmp_capabilities"}\n')
                stream.flush()
                stream.readline()
                stream.write(b'{"execute":"system_powerdown"}\n')
                stream.flush()
        except OSError:
            pass
        deadline = time.monotonic() + 15
        while alive(directory) and time.monotonic() < deadline:
            time.sleep(0.3)
        if alive(directory):
            os.kill(int((directory / "qemu.pid").read_text()), signal.SIGTERM)
    if directory.exists():
        for filename in ["qemu.pid", "forward.pid", "host-audio.pid", "qmp.sock"]:
            (directory / filename).unlink(missing_ok=True)


def upload(directory, key, port, here, data, project):
    import lab

    lab.fixtures()
    target = "/home/studio/simpleVoiceover-lab"
    ssh(directory, key, port, f"mkdir -p {target}/build/linux-testbench/fixtures")
    options = ssh_options(directory, key, port)
    options[options.index("-p")] = "-P"
    run(["scp", *options, "-r", here, f"studio@127.0.0.1:{target}/"])
    run(
        [
            "scp",
            *options,
            "-r",
            data / "fixtures",
            f"studio@127.0.0.1:{target}/build/linux-testbench/",
        ]
    )
    for kind in ["deb", "rpm", "appimage"]:
        run(["scp", *options, lab.release(kind), f"studio@127.0.0.1:{target}/build/"])
    package = {"version": json.loads((project / "package.json").read_text())["version"]}
    ssh(
        directory,
        key,
        port,
        f"printf %s {shlex.quote(json.dumps(package))} > {target}/package.json",
    )
    return target, options


def command(arguments, catalog, here, data, cache):
    name = arguments.name
    config = catalog["virtual_machines"][name]
    directory = cache / "vm" / name
    if arguments.action == "stop":
        stop(directory)
        print("VM stopped: " + name)
        return
    directory, key = prepare(name, config, cache)
    if arguments.action == "prepare":
        return
    preset = getattr(arguments, "preset", None) or config["preset"]
    if preset not in catalog["presets"]:
        raise ValueError("Unknown VM userspace preset: " + preset)
    try:
        port = start(directory, key, arguments.action == "manual")
        target, options = upload(directory, key, port, here, data, here.parent)
        if arguments.action == "auto":
            extended = " --extended" if arguments.extended else ""
            ssh(
                directory,
                key,
                port,
                f"cd {target} && linux-testbench/lab auto {shlex.quote(preset)}{extended}",
            )
        else:
            if alive(directory, "forward.pid"):
                os.kill(int((directory / "forward.pid").read_text()), signal.SIGTERM)
            ssh(directory, key, port, "systemctl --user start pulseaudio.service")
            ssh(
                directory,
                key,
                port,
                f"cd {target} && linux-testbench/lab manual {shlex.quote(preset)} --bridge host --port 6080",
            )
            browser_port = free_port()
            forward = subprocess.Popen(
                [
                    "ssh",
                    *ssh_options(directory, key, port),
                    "-o",
                    "ExitOnForwardFailure=yes",
                    "-N",
                    "-L",
                    f"127.0.0.1:{browser_port}:127.0.0.1:6080",
                    "studio@127.0.0.1",
                ],
                start_new_session=True,
            )
            (directory / "forward.pid").write_text(str(forward.pid))
            print(
                f"VM desktop: http://127.0.0.1:{browser_port}/vnc.html?autoconnect=1&resize=scale\nStop: linux-testbench/lab vm stop {name}"
            )
    finally:
        if arguments.action == "auto":
            results = data / "vm" / name
            results.mkdir(parents=True, exist_ok=True)
            try:
                if "target" in locals():
                    copied = subprocess.run(
                        [
                            "scp",
                            *options,
                            "-r",
                            f"studio@127.0.0.1:{target}/build/linux-testbench/auto",
                            str(results),
                        ],
                        check=False,
                    )
                    if copied.returncode:
                        print(
                            "No guest reports were available; inspect the VM console log",
                            flush=True,
                        )
                if (directory / "kernel.txt").exists():
                    (results / "guest-kernel.txt").write_text(
                        (directory / "kernel.txt").read_text()
                    )
            finally:
                stop(directory)
            import lab

            lab.report_index()
