import json
import os
import signal
import subprocess
import sys
import time
from pathlib import Path

RUNTIME = Path(os.environ.get("XDG_RUNTIME_DIR", "/run/studio-lab"))
PRIVATE_SERVER = "unix:" + str(RUNTIME / "pulse/native")
HOST_SERVER = os.environ.get("LAB_HOST_SERVER", "unix:/run/studio-host/pulse-native")
DEVICES = {
    "a": {"output": "lab_output_a", "input": "lab_microphone_a", "rate": 48000, "frequency": 440},
    "b": {"output": "lab_output_b", "input": "lab_microphone_b", "rate": 44100, "frequency": 660},
}


def pactl(*arguments, server=PRIVATE_SERVER):
    return subprocess.check_output(
        ["pactl", "--server=" + server, *arguments], text=True, stderr=subprocess.PIPE
    ).strip()


def devices():
    return {
        "outputs": json.loads(pactl("--format=json", "list", "sinks")),
        "inputs": json.loads(pactl("--format=json", "list", "sources")),
        "defaultOutput": pactl("get-default-sink"),
        "defaultInput": pactl("get-default-source"),
    }


def add(name):
    device = DEVICES[name]
    existing = devices()
    if not any(item["name"] == device["output"] for item in existing["outputs"]):
        pactl(
            "load-module",
            "module-null-sink",
            "sink_name=" + device["output"],
            f"rate={device['rate']}",
            "channels=2",
            "sink_properties=device.description=Lab_Output_" + name.upper(),
        )
    if not any(item["name"] == device["input"] for item in existing["inputs"]):
        fifo = RUNTIME / (device["input"] + ".pcm")
        fifo.unlink(missing_ok=True)
        pactl(
            "load-module",
            "module-pipe-source",
            "source_name=" + device["input"],
            "file=" + str(fifo),
            "format=s16le",
            f"rate={device['rate']}",
            "channels=1",
            "source_properties=device.description=Lab_Microphone_" + name.upper(),
        )


def remove(name):
    device = DEVICES[name]
    modules = pactl("list", "short", "modules").splitlines()
    for module in modules:
        columns = module.split("\t")
        arguments = columns[2] if len(columns) > 2 else ""
        if any(
            f"{key}={device[field]}" in arguments.split()
            for key, field in [("sink_name", "output"), ("source_name", "input")]
        ):
            pactl("unload-module", columns[0])


def defaults(name):
    device = DEVICES[name]
    deadline = time.monotonic() + 5
    while True:
        try:
            pactl("set-default-sink", device["output"])
            pactl("set-default-source", device["input"])
            return
        except subprocess.CalledProcessError:
            if time.monotonic() >= deadline:
                raise
            time.sleep(0.1)


def initialize():
    add("a")
    add("b")
    defaults("a")


def follow_host_defaults(identifier):
    for kind, destination, operation, field in [
        ("sink-inputs", "get-default-sink", "move-sink-input", "sink"),
        ("source-outputs", "get-default-source", "move-source-output", "source"),
    ]:
        target = pactl(destination, server=HOST_SERVER)
        devices_kind = "sinks" if field == "sink" else "sources"
        targets = pactl("list", "short", devices_kind, server=HOST_SERVER).splitlines()
        target_id = next(line.split("\t")[0] for line in targets if line.split("\t")[1] == target)
        streams = json.loads(pactl("--format=json", "list", kind, server=HOST_SERVER))
        indices = {
            line.split("\t")[2]: line.split("\t")[0]
            for line in pactl("list", "short", kind, server=HOST_SERVER).splitlines()
        }
        for stream in streams:
            if (
                stream.get("properties", {}).get("application.id") == identifier
                and str(stream.get(field)) != target_id
            ):
                index = stream["index"] if "index" in stream else indices[str(stream["client"])]
                pactl(operation, str(index), target, server=HOST_SERVER)


def follow(identifier, path):
    running = True

    def finish(*_):
        nonlocal running
        running = False

    signal.signal(signal.SIGTERM, finish)
    signal.signal(signal.SIGINT, finish)
    with Path(path).open("a") as log:
        while running:
            try:
                follow_host_defaults(identifier)
            except (
                subprocess.CalledProcessError,
                json.JSONDecodeError,
                OSError,
                StopIteration,
                IndexError,
                KeyError,
            ) as error:
                print(str(error), file=log, flush=True)
            time.sleep(1)


def spawn(arguments, log, **options):
    return subprocess.Popen(arguments, stderr=log, start_new_session=True, **options)


def stop(process):
    if process.poll() is None:
        try:
            os.killpg(process.pid, signal.SIGTERM)
        except ProcessLookupError:
            process.wait()
            return
        try:
            process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            process.wait()


def serve(bridge):
    workers = {}
    running = True
    identifier = "dev.simplevoiceover.linux-testbench." + os.environ.get("LAB_PROFILE", "manual")

    def finish(*_):
        nonlocal running
        running = False

    signal.signal(signal.SIGTERM, finish)
    signal.signal(signal.SIGINT, finish)
    with open("/results/audio-bridge.log", "a") as log:
        try:
            while running:
                try:
                    catalog = devices()
                    wanted = set()
                    for name, device in DEVICES.items():
                        if any(item["name"] == device["input"] for item in catalog["inputs"]):
                            key = "input_" + name
                            wanted.add(key)
                            if key not in workers or any(
                                process.poll() is not None for process in workers[key]
                            ):
                                for process in workers.pop(key, []):
                                    stop(process)
                                fifo = str(RUNTIME / (device["input"] + ".pcm"))
                                if bridge == "host":
                                    environment = dict(os.environ, LAB_INPUT_FIFO=fifo)
                                    command = [
                                        "bash",
                                        "-c",
                                        'exec "$@" > "$LAB_INPUT_FIFO"',
                                        "bash",
                                        "parec",
                                        "--server=" + HOST_SERVER,
                                        "--device=@DEFAULT_SOURCE@",
                                        "--format=s16le",
                                        f"--rate={device['rate']}",
                                        "--channels=1",
                                        "--property=application.id=" + identifier,
                                        "--stream-name=Lab_Microphone_" + name.upper(),
                                    ]
                                    workers[key] = [
                                        spawn(
                                            command, log, stdout=subprocess.DEVNULL, env=environment
                                        )
                                    ]
                                else:
                                    command = [
                                        "ffmpeg",
                                        "-nostdin",
                                        "-hide_banner",
                                        "-loglevel",
                                        "error",
                                        "-y",
                                        "-re",
                                        "-f",
                                        "lavfi",
                                        "-i",
                                        f"sine=frequency={device['frequency']}:sample_rate={device['rate']}",
                                        "-ac",
                                        "1",
                                        "-f",
                                        "s16le",
                                        fifo,
                                    ]
                                    workers[key] = [spawn(command, log, stdout=subprocess.DEVNULL)]
                        if bridge == "host" and any(
                            item["name"] == device["output"] for item in catalog["outputs"]
                        ):
                            key = "output_" + name
                            wanted.add(key)
                            if key not in workers or any(
                                process.poll() is not None for process in workers[key]
                            ):
                                for process in workers.pop(key, []):
                                    stop(process)
                                producer = spawn(
                                    [
                                        "parec",
                                        "--server=" + PRIVATE_SERVER,
                                        "--device=" + device["output"] + ".monitor",
                                        "--format=float32le",
                                        "--rate=48000",
                                        "--channels=2",
                                    ],
                                    log,
                                    stdout=subprocess.PIPE,
                                )
                                consumer = spawn(
                                    [
                                        "pacat",
                                        "--server=" + HOST_SERVER,
                                        "--playback",
                                        "--format=float32le",
                                        "--rate=48000",
                                        "--channels=2",
                                        "--property=application.id=" + identifier,
                                        "--stream-name=Lab_Output_" + name.upper(),
                                    ],
                                    log,
                                    stdin=producer.stdout,
                                    stdout=subprocess.DEVNULL,
                                )
                                producer.stdout.close()
                                workers[key] = [producer, consumer]
                    for key in set(workers) - wanted:
                        for process in workers.pop(key):
                            stop(process)
                    if bridge == "host":
                        follow_host_defaults(identifier)
                except (
                    subprocess.CalledProcessError,
                    json.JSONDecodeError,
                    OSError,
                    StopIteration,
                    IndexError,
                    KeyError,
                ) as error:
                    print(str(error), file=log, flush=True)
                time.sleep(1)
        finally:
            for group in workers.values():
                for process in group:
                    stop(process)


if __name__ == "__main__":
    action = sys.argv[1]
    if action == "serve":
        serve(sys.argv[2])
    elif action == "follow":
        follow(sys.argv[2], sys.argv[3])
    elif action == "initialize":
        initialize()
    elif action == "devices":
        print(json.dumps(devices(), indent=2))
    elif action == "default":
        defaults(sys.argv[2])
    elif action == "remove":
        remove(sys.argv[2])
    elif action == "restore":
        add(sys.argv[2])
    else:
        raise SystemExit("Unknown audio action: " + action)
