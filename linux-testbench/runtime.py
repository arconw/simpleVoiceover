import json
import os
import shutil
import signal
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

import audio


def install_artifact():
    artifact = Path("/artifact")
    kind = os.environ["LAB_ARTIFACT"]
    if kind == "deb":
        subprocess.run(["apt-get", "update"], check=True)
        subprocess.run(
            ["apt-get", "install", "--yes", "/artifact/release.deb"],
            check=True,
            env=dict(os.environ, DEBIAN_FRONTEND="noninteractive"),
        )
        executable = subprocess.check_output(
            ["bash", "-c", "command -v simpleVoiceover"], text=True
        ).strip()
    elif kind == "rpm":
        subprocess.run(["dnf", "install", "--assumeyes", "/artifact/release.rpm"], check=True)
        executable = subprocess.check_output(
            ["bash", "-c", "command -v simpleVoiceover"], text=True
        ).strip()
    else:
        directory = Path("/opt/studio-appimage")
        directory.mkdir(exist_ok=True)
        subprocess.run(
            [str(artifact / "release.AppImage"), "--appimage-extract"],
            cwd=directory,
            check=True,
            stdout=subprocess.DEVNULL,
        )
        subprocess.run(["chown", "--recursive", "studio:studio", str(directory)], check=True)
        if os.environ.get("LAB_WEBKIT", "bundled") == "system":
            source = directory / "squashfs-root/usr/bin/simpleVoiceover"
            if not source.is_file():
                raise RuntimeError("AppImage's application ELF was not found")
            isolated = Path("/opt/studio-system")
            isolated.mkdir(exist_ok=True)
            executable = str(isolated / "simpleVoiceover")
            shutil.copy2(source, executable)
        else:
            executable = str(directory / "squashfs-root/AppRun")
    Path("/opt/studio-executable").write_text(executable)
    os.execvpe(
        "runuser",
        [
            "runuser",
            "-u",
            "studio",
            "--",
            "python3",
            "-B",
            "/opt/linux-testbench/runtime.py",
            "session",
        ],
        os.environ,
    )


def session():
    processes = []
    names = {}
    handles = []
    running = True

    def finish(*_):
        nonlocal running
        running = False

    signal.signal(signal.SIGTERM, finish)
    signal.signal(signal.SIGINT, finish)
    result = Path("/results")
    runtime = Path(os.environ["XDG_RUNTIME_DIR"])
    runtime.mkdir(mode=0o700, exist_ok=True)
    os.chmod(runtime, 0o700)
    (runtime / "pulse").mkdir(exist_ok=True)

    def launch(name, arguments, environment=None):
        handle = open(result / (name + ".log"), "a")
        handles.append(handle)
        process = subprocess.Popen(
            arguments, stdout=handle, stderr=handle, env=environment, start_new_session=True
        )
        processes.append(process)
        names[process.pid] = name
        return process

    try:
        bus = subprocess.check_output(
            ["dbus-daemon", "--session", "--fork", "--print-address=1", "--print-pid=1"], text=True
        ).splitlines()
        os.environ["DBUS_SESSION_BUS_ADDRESS"] = bus[0]
        os.environ["DISPLAY"] = ":99"
        os.environ["GDK_BACKEND"] = "x11"
        launch("xvfb", ["Xvfb", ":99", "-screen", "0", "1440x900x24", "-nolisten", "tcp"])
        time.sleep(0.4)
        launch("window-manager", ["openbox"])
        if os.environ.get("LAB_DISPLAY") == "wayland":
            launch(
                "weston",
                [
                    "weston",
                    "--backend=x11-backend.so",
                    "--socket=wayland-lab",
                    "--width=1440",
                    "--height=900",
                    "--idle-time=0",
                ],
            )
            deadline = time.monotonic() + 15
            while not (runtime / "wayland-lab").exists():
                if time.monotonic() > deadline:
                    raise RuntimeError("Nested Wayland compositor did not start")
                time.sleep(0.1)
            os.environ["WAYLAND_DISPLAY"] = "wayland-lab"
            os.environ["GDK_BACKEND"] = "wayland"
        if os.environ["LAB_AUDIO"] == "pulseaudio":
            configuration = runtime / "default.pa"
            configuration.write_text(
                f"load-module module-native-protocol-unix socket={runtime}/pulse/native auth-anonymous=1\n"
            )
            launch(
                "pulseaudio",
                [
                    "pulseaudio",
                    "-n",
                    "--daemonize=no",
                    "--exit-idle-time=-1",
                    "--use-pid-file=no",
                    "--disallow-exit",
                    "--file=" + str(configuration),
                    "--log-target=stderr",
                ],
            )
        else:
            if Path("/usr/share/wireplumber/bluetooth.lua.d").is_dir():
                configuration = Path.home() / ".config/wireplumber/bluetooth.lua.d"
                configuration.mkdir(parents=True, exist_ok=True)
                (configuration / "51-testbench-headless.lua").write_text(
                    'if bluez_monitor then bluez_monitor.properties["with-logind"] = false end\nif bluez_midi_monitor then bluez_midi_monitor.properties["with-logind"] = false end\n'
                )
            launch("pipewire", ["pipewire"])
            launch("wireplumber", ["wireplumber"])
            launch("pipewire-pulse", ["pipewire-pulse"])
        deadline = time.monotonic() + 30
        while True:
            try:
                audio.pactl("info")
                break
            except subprocess.CalledProcessError:
                if time.monotonic() > deadline:
                    raise RuntimeError("Private audio server did not start; inspect server logs")
                time.sleep(0.2)
        audio.initialize()
        launch(
            "audio-workers",
            [
                "python3",
                "-B",
                "/opt/linux-testbench/audio.py",
                "serve",
                os.environ.get("LAB_BRIDGE", "virtual"),
            ],
        )
        launch(
            "vnc",
            [
                "x11vnc",
                "-display",
                ":99",
                "-rfbport",
                "5900",
                "-listen",
                "0.0.0.0",
                "-forever",
                "-shared",
                "-nopw",
            ],
            environment={
                key: value
                for key, value in dict(
                    os.environ, GDK_BACKEND="x11", XDG_SESSION_TYPE="x11"
                ).items()
                if key != "WAYLAND_DISPLAY"
            },
        )
        web = next(
            path for path in [Path("/usr/share/novnc"), Path("/usr/share/noVNC")] if path.exists()
        )
        launch("novnc", ["websockify", "--web=" + str(web), "0.0.0.0:6080", "127.0.0.1:5900"])
        os.environ["TAURI_WEBVIEW_AUTOMATION"] = "true"
        tool = Path("/tools/webdriver")
        driver = (
            [
                str(tool / "ld-linux-x86-64.so.2"),
                "--library-path",
                str(tool / "lib"),
                str(tool / "WebKitWebDriver"),
            ]
            if tool.exists()
            else ["WebKitWebDriver"]
        )
        launch("webdriver", driver + ["--port=4444"])
        deadline = time.monotonic() + 15
        while True:
            try:
                with urllib.request.urlopen("http://127.0.0.1:4444/status", timeout=1):
                    break
            except (urllib.error.URLError, TimeoutError):
                if time.monotonic() > deadline:
                    raise RuntimeError("WebKit WebDriver did not start; inspect webdriver.log")
                time.sleep(0.1)
        environment = {
            key: value
            for key, value in os.environ.items()
            if key
            in [
                "HOME",
                "XDG_RUNTIME_DIR",
                "PULSE_SERVER",
                "DBUS_SESSION_BUS_ADDRESS",
                "DISPLAY",
                "GDK_BACKEND",
                "WAYLAND_DISPLAY",
                "TAURI_WEBVIEW_AUTOMATION",
                "LANG",
                "PATH",
                "LAB_PROFILE",
                "LAB_AUDIO",
                "LAB_ARTIFACT",
                "LAB_WEBKIT",
                "LAB_BRIDGE",
                "LAB_CODECS",
            ]
        }
        (result / "session-environment.json").write_text(json.dumps(environment, indent=2))
        (result / "ready").write_text("ready\n")
        if os.environ.get("LAB_MANUAL") == "1":
            launch(
                "application",
                [
                    Path("/opt/studio-executable").read_text(),
                    "--port",
                    "5174",
                    "--config-dir",
                    str(result / "manual-settings"),
                ],
            )
            launch(
                "controls",
                [
                    "xterm",
                    "-title",
                    "Linux test bench controls",
                    "-e",
                    "bash",
                    "--rcfile",
                    "/opt/linux-testbench/interactive.bash",
                ],
            )
        while running:
            critical = [process for process in processes if process.poll() is not None]
            if critical:
                raise RuntimeError(
                    "A desktop/audio service exited: "
                    + ", ".join(
                        f"{names[process.pid]} ({process.returncode})" for process in critical
                    )
                    + "; inspect its log"
                )
            time.sleep(0.5)
    finally:
        (result / "ready").unlink(missing_ok=True)
        for process in reversed(processes):
            audio.stop(process)
        if "bus" in locals():
            try:
                os.kill(int(bus[1]), signal.SIGTERM)
            except ProcessLookupError:
                pass
        for handle in handles:
            handle.close()


if __name__ == "__main__":
    if len(sys.argv) == 1:
        install_artifact()
    else:
        session()
