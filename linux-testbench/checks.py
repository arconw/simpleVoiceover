import array
import base64
import json
import math
import os
import re
import subprocess
import sys
import time
import traceback
import urllib.error
import urllib.request
import wave
import xml.etree.ElementTree as ET
from pathlib import Path

import audio
import measurements
import native_input

RESULTS = Path(os.environ.get("LAB_RESULTS", "/results"))
FIXTURES = Path(os.environ.get("LAB_FIXTURES", "/fixtures"))
EXECUTABLE = os.environ.get("LAB_EXECUTABLE") or Path("/opt/studio-executable").read_text()


class Driver:
    def __init__(self):
        self.session = None
        self.native_pointer = None

    def request(self, method, path, payload=None):
        body = json.dumps(payload).encode() if payload is not None else None
        request = urllib.request.Request(
            "http://127.0.0.1:4444" + path,
            data=body,
            method=method,
            headers={"Content-Type": "application/json"},
        )
        try:
            with urllib.request.urlopen(request, timeout=90) as response:
                result = json.load(response)
        except urllib.error.HTTPError as error:
            raise RuntimeError(error.read().decode()) from error
        value = result.get("value", {})
        if isinstance(value, dict) and value.get("error") and "ok" not in value:
            raise RuntimeError(json.dumps(value))
        return value

    def start(self):
        settings = RESULTS / "auto-settings"
        settings.mkdir(parents=True, exist_ok=True)
        result = self.request(
            "POST",
            "/session",
            {
                "capabilities": {
                    "alwaysMatch": {
                        "webkitgtk:browserOptions": {
                            "binary": EXECUTABLE,
                            "args": ["--port", "5174", "--config-dir", str(settings)],
                        }
                    }
                }
            },
        )
        self.session = result["sessionId"]
        self.request(
            "POST", self.path("/timeouts"), {"script": 60000, "pageLoad": 60000, "implicit": 0}
        )
        return self.wait(
            "return !!window.__TAURI_INTERNALS__ && !!document.querySelector('.play-button');"
        )

    def path(self, suffix):
        return "/session/" + self.session + suffix

    def js(self, body):
        script = (
            "const done=arguments[arguments.length-1];(async()=>{"
            + body
            + "})().then(value=>done({ok:true,value}),error=>done({ok:false,error:String(error)}));"
        )
        result = self.request("POST", self.path("/execute/async"), {"script": script, "args": []})
        if not result["ok"]:
            raise RuntimeError(result["error"])
        return result.get("value")

    def wait(self, body, timeout=20):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            value = self.js(body)
            if value:
                return value
            time.sleep(0.15)
        raise AssertionError("Timed out waiting for: " + body)

    def ipc(self, command, **payload):
        request = {"command": command, "language": "en", **payload}
        return self.js(
            "return await window.__TAURI_INTERNALS__.invoke('studio_command',{request:"
            + json.dumps(request)
            + "});"
        )

    def click(self, selector):
        element = self.request(
            "POST", self.path("/element"), {"using": "css selector", "value": selector}
        )
        identifier = element["element-6066-11e4-a52e-4f735466cecf"]
        try:
            self.request("POST", self.path("/element/" + identifier + "/click"), {})
        except RuntimeError as error:
            if "unsupported operation" not in str(error):
                raise
            geometry = self.js(
                "const element=document.querySelector("
                + json.dumps(selector)
                + ");element.scrollIntoView({block:'nearest'});const box=element.getBoundingClientRect();return {x:box.left+box.width/2,y:box.top+box.height/2};"
            )
            self.native_click(geometry["x"], geometry["y"])

    def pointer(self, x, y):
        try:
            self.webdriver_pointer(x, y)
        except RuntimeError as error:
            if "unsupported operation" not in str(error):
                raise
            self.native_click(x, y)

    def native_click(self, x, y):
        if self.native_pointer is None:
            self.native_pointer = native_input.Pointer()
            (RESULTS / "pointer.json").write_text(
                json.dumps({"mode": "local VNC", "reason": "WebDriver mouse unsupported"}) + "\n"
            )
        origin = self.js("return {x:window.screenX,y:window.screenY};")
        self.native_pointer.click(round(origin["x"] + x), round(origin["y"] + y))
        time.sleep(0.06)

    def webdriver_pointer(self, x, y):
        self.request(
            "POST",
            self.path("/actions"),
            {
                "actions": [
                    {
                        "type": "pointer",
                        "id": "lab-mouse",
                        "parameters": {"pointerType": "mouse"},
                        "actions": [
                            {
                                "type": "pointerMove",
                                "duration": 0,
                                "origin": "viewport",
                                "x": round(x),
                                "y": round(y),
                            },
                            {"type": "pointerDown", "button": 0},
                            {"type": "pointerUp", "button": 0},
                        ],
                    }
                ]
            },
        )

    def fresh(self):
        if self.js("return !!document.querySelector('.recording-badge');"):
            self.click(".record-button")
            self.wait("return !document.querySelector('.recording-badge');")
        if self.js(
            "return document.querySelector('.play-button')?.getAttribute('aria-label')==='Pause';"
        ):
            self.click(".play-button")
            self.wait(
                "return document.querySelector('.play-button')?.getAttribute('aria-label')!=='Pause';"
            )
        self.ipc("pause")
        snapshot = self.ipc("snapshot")
        for track in snapshot["project"]["tracks"]:
            self.ipc("track_patch", trackId=track["id"], patch={"locked": False})
            self.ipc("track_remove", trackId=track["id"])
        for asset in snapshot["project"]["assets"]:
            self.ipc("asset_remove", assetId=asset["id"])
        snapshot = self.ipc("track_add")
        track = snapshot["project"]["tracks"][0]["id"]
        self.ipc(
            "track_patch",
            trackId=track,
            patch={
                "name": "Procedural test",
                "volume": 0,
                "pan": 0,
                "mute": False,
                "solo": False,
                "fxBypass": True,
                "armed": True,
            },
        )
        self.ipc("audio_preferences_patch", inputDevice="", outputDevice="")
        self.js(
            "document.querySelector('.tl-ruler').dispatchEvent(new KeyboardEvent('keydown',{key:'Home',bubbles:true}));return true;"
        )
        self.wait(
            "return Number(document.querySelector('.tl-ruler').getAttribute('aria-valuenow'))<0.01;"
        )
        return track

    def screenshot(self, name):
        if self.session:
            try:
                content = self.request("GET", self.path("/screenshot"))
                (RESULTS / (name + ".png")).write_bytes(base64.b64decode(content))
            except Exception:
                pass

    def close(self):
        if self.native_pointer is not None:
            self.native_pointer.close()
            self.native_pointer = None
        if self.session:
            try:
                self.ipc("pause")
                self.js(
                    "await window.__TAURI_INTERNALS__.invoke('close_studio',{withoutSaving:true});return true;"
                )
            except Exception:
                pass
            try:
                self.request("DELETE", self.path(""))
            except Exception:
                pass
            self.session = None


def analyze(path, expected_duration=None, frequency=None):
    metadata = json.loads(
        subprocess.check_output(
            ["ffprobe", "-v", "error", "-show_streams", "-show_format", "-of", "json", str(path)],
            text=True,
        )
    )
    stream = next(item for item in metadata["streams"] if item["codec_type"] == "audio")
    duration = float(metadata["format"]["duration"])
    if expected_duration is not None:
        assert abs(duration - expected_duration) < 0.3, (duration, expected_duration)
    pcm = subprocess.check_output(
        [
            "ffmpeg",
            "-nostdin",
            "-v",
            "error",
            "-i",
            str(path),
            "-t",
            "2",
            "-f",
            "f32le",
            "-ac",
            "1",
            "-ar",
            "16000",
            "pipe:1",
        ]
    )
    samples = array.array("f", pcm)
    assert samples and all(math.isfinite(value) for value in samples), "Invalid decoded PCM"
    peak = max(abs(value) for value in samples)
    rms = math.sqrt(sum(value * value for value in samples) / len(samples))
    assert peak > 0.001 and rms > 0.0001, ("Silent audio", peak, rms)
    assert peak <= 1.01, ("Clipped or invalid output", peak)
    tone = None
    if frequency:
        selected = samples[len(samples) // 2 :]
        tone = measurements.tone_amplitude(selected, 16000, frequency)
        assert tone > rms * 0.4, ("Expected tone missing", frequency, tone, rms)
    return {
        "codec": stream["codec_name"],
        "sampleRate": stream["sample_rate"],
        "channels": stream["channels"],
        "duration": duration,
        "peak": peak,
        "rms": rms,
        "toneAmplitude": tone,
    }


def capture(device):
    process = subprocess.Popen(
        [
            "parec",
            "--server=" + audio.PRIVATE_SERVER,
            "--device=" + device,
            "--format=float32le",
            "--rate=48000",
            "--channels=2",
            "--latency-msec=40",
        ],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    try:
        pcm, error = process.communicate(timeout=1.2)
    except subprocess.TimeoutExpired:
        process.terminate()
        pcm, error = process.communicate(timeout=5)
    samples = array.array("f", pcm[: len(pcm) // 4 * 4])
    assert len(samples) >= 4800, ("Insufficient captured PCM", len(samples), error.decode())
    mono = samples[::2]
    tones = {
        str(frequency): 2
        * math.hypot(
            sum(
                value * math.cos(2 * math.pi * frequency * index / 48000)
                for index, value in enumerate(mono)
            ),
            sum(
                value * math.sin(2 * math.pi * frequency * index / 48000)
                for index, value in enumerate(mono)
            ),
        )
        / len(mono)
        for frequency in [440, 660]
    }
    return {
        "samples": len(samples),
        "peak": max(map(abs, samples)),
        "rms": math.sqrt(sum(value * value for value in samples) / len(samples)),
        "tones": tones,
    }


def imports_and_exports(driver, fixture):
    track = driver.fresh()
    snapshot = driver.ipc(
        "import_paths", paths=[str(FIXTURES / fixture["file"])], trackId=track, position=0
    )
    assert len(snapshot["project"]["assets"]) == 1
    assert abs(snapshot["duration"] - fixture["duration"]) < 0.3
    evidence = {}
    for kind in ["wav", "mp3"]:
        path = RESULTS / (Path(fixture["file"]).stem + "-export." + kind)
        result = driver.ipc("export", trackId=track, format=kind, path=str(path))
        assert result["saved"] and path.is_file()
        evidence[kind] = analyze(path, fixture["duration"], fixture["frequency"])
        assert evidence[kind]["codec"] == ("pcm_s16le" if kind == "wav" else "mp3")
    return evidence


def mixed_project(driver):
    track_a = driver.fresh()
    driver.ipc(
        "import_paths", paths=[str(FIXTURES / "tone-mono-44100.wav")], trackId=track_a, position=0
    )
    previous = {item["id"] for item in driver.ipc("snapshot")["project"]["tracks"]}
    added = driver.ipc("track_add")
    track_b = next(item["id"] for item in added["project"]["tracks"] if item["id"] not in previous)
    driver.ipc(
        "track_patch",
        trackId=track_b,
        patch={
            "name": "Second tone",
            "volume": 0,
            "pan": 0,
            "mute": False,
            "solo": False,
            "fxBypass": True,
        },
    )
    driver.ipc(
        "import_paths",
        paths=[str(FIXTURES / "tone-stereo-48000-24bit.wav")],
        trackId=track_b,
        position=0,
    )
    evidence = {}
    for kind in ["wav", "mp3"]:
        path = RESULTS / ("mixed-project." + kind)
        assert driver.ipc("export", format=kind, path=str(path))["saved"]
        evidence[kind] = [analyze(path, 8, frequency) for frequency in [440, 660]]
    project_path = RESULTS / "roundtrip.svoice"
    before = driver.ipc("snapshot")
    assert driver.ipc("save", path=str(project_path))["saved"]
    driver.ipc("track_patch", trackId=track_b, patch={"name": "Changed after saving"})
    reopened = driver.ipc("open", path=str(project_path))
    assert reopened["project"]["tracks"] == before["project"]["tracks"]
    assert len(reopened["project"]["assets"]) == 2
    path = RESULTS / "reopened-project.wav"
    driver.ipc("export", format="wav", path=str(path))
    evidence["reopened"] = [analyze(path, 8, frequency) for frequency in [440, 660]]
    return evidence


def playback(driver, position=0):
    if position:
        driver.js(
            "document.querySelector('.tl-ruler').dispatchEvent(new KeyboardEvent('keydown',{key:'Home',bubbles:true}));return true;"
        )
    if driver.js(
        "return document.querySelector('.play-button').getAttribute('aria-label')!=='Pause';"
    ):
        driver.click(".play-button")
    driver.wait(
        "return document.querySelector('.play-button').getAttribute('aria-label')==='Pause';"
    )
    time.sleep(0.5)


def routing(driver):
    audio.add("a")
    audio.add("b")
    audio.defaults("a")
    track = driver.fresh()
    for position in [0, 8, 16, 24]:
        driver.ipc(
            "import_paths",
            paths=[str(FIXTURES / "tone-mono-44100.wav")],
            trackId=track,
            position=position,
        )
    playback(driver)
    evidence = {"defaultA": capture("lab_output_a.monitor")}
    assert evidence["defaultA"]["peak"] > 0.001
    audio.defaults("b")
    time.sleep(2.5)
    evidence["defaultB"] = capture("lab_output_b.monitor")
    assert evidence["defaultB"]["peak"] > 0.001
    driver.ipc(
        "audio_preferences_patch", inputDevice="lab_microphone_a", outputDevice="lab_output_a"
    )
    driver.ipc("audio_devices_sync")
    time.sleep(1)
    evidence["pinnedA"] = capture("lab_output_a.monitor")
    assert evidence["pinnedA"]["peak"] > 0.001
    driver.ipc("audio_preferences_patch", inputDevice="", outputDevice="lab_output_b")
    audio.defaults("a")
    audio.remove("b")
    time.sleep(2.5)
    evidence["fallbackA"] = capture("lab_output_a.monitor")
    assert evidence["fallbackA"]["peak"] > 0.001
    audio.add("b")
    time.sleep(2.5)
    evidence["restoredB"] = capture("lab_output_b.monitor")
    assert evidence["restoredB"]["peak"] > 0.001
    driver.click(".play-button")
    return evidence


def microphone(driver, name):
    device = audio.DEVICES[name]
    audio.defaults(name)
    track = driver.fresh()
    driver.ipc("audio_preferences_patch", inputDevice=device["input"], outputDevice="")
    time.sleep(2.5)
    driver.click(".record-button")
    driver.wait("return !!document.querySelector('.recording-badge');")
    time.sleep(2)
    driver.click(".record-button")
    driver.wait("return !document.querySelector('.recording-badge');")
    snapshot = driver.ipc("snapshot")
    recorded = next(item for item in snapshot["project"]["tracks"] if item["id"] == track)
    assert recorded["clips"], "No recording was saved"
    path = RESULTS / ("microphone-" + name + ".wav")
    driver.ipc("export", trackId=track, format="wav", path=str(path))
    return analyze(path, frequency=device["frequency"])


def microphone_switch(driver):
    audio.defaults("a")
    track = driver.fresh()
    time.sleep(2.5)
    driver.click(".record-button")
    driver.wait("return !!document.querySelector('.recording-badge');")
    time.sleep(2)
    audio.defaults("b")
    time.sleep(4)
    driver.click(".record-button")
    driver.wait("return !document.querySelector('.recording-badge');")
    path = RESULTS / "microphone-live-switch.wav"
    driver.ipc("export", trackId=track, format="wav", path=str(path))
    with wave.open(str(path)) as recording:
        parameters = recording.getparams()
        frames = recording.readframes(recording.getnframes())
    stride = parameters.nchannels * parameters.sampwidth
    second = parameters.framerate * stride
    assert len(frames) >= second * 4, "Recording was interrupted during device switching"
    evidence = {}
    for label, samples, frequency in [
        ("before", frames[second // 2 : second * 3 // 2], 440),
        ("after", frames[-second:], 660),
    ]:
        excerpt = RESULTS / ("microphone-live-switch-" + label + ".wav")
        with wave.open(str(excerpt), "wb") as recording:
            recording.setparams(parameters)
            recording.writeframes(samples)
        evidence[label] = analyze(excerpt, 1, frequency)
    return evidence


def observe_frames(driver):
    driver.js("""
const video=document.querySelector('video');
window.__labFrames?.stop();
const state={count:0,last:null,previous:video.getVideoPlaybackQuality().totalVideoFrames};
window.__labFrames=state;
if(typeof video.requestVideoFrameCallback==='function'){
  const next=(_now,metadata)=>{state.count++;state.last=metadata.mediaTime;state.id=video.requestVideoFrameCallback(next);};
  state.id=video.requestVideoFrameCallback(next);
  state.stop=()=>video.cancelVideoFrameCallback(state.id);
}else{
  const timer=setInterval(()=>{const frames=video.getVideoPlaybackQuality().totalVideoFrames;state.count+=frames>=state.previous?frames-state.previous:frames;state.previous=frames;},50);
  state.stop=()=>clearInterval(timer);
}
return true;
""")


def video(driver, fixture):
    track = driver.fresh()
    snapshot = driver.ipc(
        "import_paths", paths=[str(FIXTURES / fixture["file"])], trackId=track, position=0
    )
    assert snapshot["project"]["assets"][0]["kind"] == "video"
    asset_id = snapshot["project"]["assets"][0]["id"]
    driver.wait(
        "const video=document.querySelector('video');if(!video?.getAttribute('src')?.endsWith("
        + json.dumps(asset_id)
        + "))return false;if(video.error)throw new Error(video.error.message);return video.readyState>=2;",
        timeout=30,
    )
    playback(driver)
    observe_frames(driver)
    before = driver.js(
        "const video=document.querySelector('video');return {time:video.currentTime,position:Number(document.querySelector('.tl-ruler').getAttribute('aria-valuenow')),frames:video.getVideoPlaybackQuality().totalVideoFrames};"
    )
    time.sleep(1.2)
    after = driver.js(
        "const video=document.querySelector('video');window.__labFrames.stop();return {time:video.currentTime,position:Number(document.querySelector('.tl-ruler').getAttribute('aria-valuenow')),frames:window.__labFrames.count,frameTime:window.__labFrames.last,error:video.error?.message};"
    )
    assert after["position"] > before["position"] + 0.5, (before, after)
    assert after["frames"] > 2 and not after.get("error"), after
    audio_evidence = capture(audio.devices()["defaultOutput"] + ".monitor")
    if fixture["kind"] == "video-only":
        assert audio_evidence["peak"] < 0.0001, audio_evidence
    else:
        assert audio_evidence["peak"] > 0.001
    driver.click(".play-button")
    driver.wait(
        "return document.querySelector('.play-button').getAttribute('aria-label')!=='Pause';"
    )
    return {"before": before, "after": after, "audio": audio_evidence}


def scrubbing(driver, filename="h264-aac-long-gop.mp4"):
    fixture = {"file": filename, "kind": "video", "frequency": 440, "duration": 12}
    video(driver, fixture)
    playback(driver)
    driver.js("""
const video=document.querySelector('video');
window.__labClock=[];
const mark=kind=>window.__labClock.push({kind,now:performance.now(),position:Number(document.querySelector('.tl-ruler').getAttribute('aria-valuenow')),time:video.currentTime});
for(const event of ['seeking','seeked','playing','pause'])video.addEventListener(event,()=>mark(event));
return true;
""")
    geometry = driver.js("""
const ruler=document.querySelector('.tl-ruler');
const box=ruler.getBoundingClientRect(),maximum=Number(ruler.getAttribute('aria-valuemax'))||60;
window.__labSeek={bad:[],target:0,changed:0};
const mark=event=>{window.__labSeek.target=(event.clientX-box.left)/box.width*maximum;window.__labSeek.changed=performance.now();};
ruler.addEventListener('pointerdown',mark);
const observer=new MutationObserver(()=>{const state=window.__labSeek,value=Number(ruler.getAttribute('aria-valuenow'));if(state.changed&&value<state.target-.3)state.bad.push(value);});
observer.observe(ruler,{attributes:true,attributeFilter:['aria-valuenow']});
window.__labSeek.cleanup=()=>{observer.disconnect();ruler.removeEventListener('pointerdown',mark);};
return {left:box.left,top:box.top,width:box.width,maximum};
""")
    for position in [2, 8] * 8 + [4]:
        driver.pointer(
            geometry["left"] + position / geometry["maximum"] * geometry["width"],
            geometry["top"] + 8,
        )
        time.sleep(0.04)
    driver.js("await new Promise(resolve=>setTimeout(resolve,1500));return true;")
    observe_frames(driver)
    evidence = driver.js("""
const video=document.querySelector('video'),ruler=document.querySelector('.tl-ruler');
const before=video.getVideoPlaybackQuality(),started=performance.now();
await new Promise(resolve=>setTimeout(resolve,2000));
window.__labFrames.stop();
const after=video.getVideoPlaybackQuality();window.__labSeek.cleanup();
return {time:video.currentTime,seeking:video.seeking,paused:video.paused,frameTime:window.__labFrames.last,width:video.videoWidth,height:video.videoHeight,position:Number(ruler.getAttribute('aria-valuenow')),seconds:(performance.now()-started)/1000,frames:window.__labFrames.count,dropped:Math.max(0,after.droppedVideoFrames-before.droppedVideoFrames),badPositions:window.__labSeek.bad,clock:window.__labClock,error:video.error?.message};
""")
    evidence["deliveredFps"] = evidence["frames"] / evidence["seconds"]
    assert not evidence.get("error") and evidence["deliveredFps"] >= 10, evidence
    assert abs(evidence["time"] - evidence["position"]) < 0.35, evidence
    if evidence["frameTime"] is not None:
        assert abs(evidence["frameTime"] - evidence["position"]) < 0.35, evidence
    assert not evidence["badPositions"], evidence
    driver.click(".play-button")
    driver.wait(
        "return document.querySelector('.play-button')?.getAttribute('aria-label')!=='Pause';"
    )
    return evidence


def fullscreen(driver):
    driver.js(
        "document.querySelector('.video-stage').dispatchEvent(new MouseEvent('dblclick',{bubbles:true,button:0}));return true;"
    )
    driver.wait("return !!document.querySelector('.preview-panel.is-fullscreen');")
    assert driver.js(
        "return await window.__TAURI_INTERNALS__.invoke('plugin:window|is_fullscreen',{label:'main'});"
    )
    driver.js(
        "document.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true}));return true;"
    )
    driver.wait("return !document.querySelector('.preview-panel.is-fullscreen');")
    assert not driver.js(
        "return await window.__TAURI_INTERNALS__.invoke('plugin:window|is_fullscreen',{label:'main'});"
    )
    return {"nativeFullscreen": "entered and exited; application still responds"}


def multistream(driver, fixture):
    driver.fresh()
    before = driver.ipc("snapshot")
    original = before["project"]["tracks"][0]["id"]
    snapshot = driver.ipc("import_paths", paths=[str(FIXTURES / fixture["file"])], position=0)
    tracks = snapshot["project"]["tracks"]
    assert [track["kind"] for track in tracks[:4]] == ["video", "audio", "audio", "audio"]
    assert tracks[4]["id"] == original and tracks[4]["armed"]
    assert len(snapshot["project"]["assets"]) == 4
    assert [track["mute"] for track in tracks[1:4]] == [False, True, True]
    evidence = []
    for track, frequency in zip(tracks[1:4], [440, 660, 880]):
        path = RESULTS / f"{fixture['file']}-{frequency}.wav"
        driver.ipc("export", trackId=track["id"], format="wav", path=str(path))
        evidence.append(analyze(path, frequency=frequency))
        if frequency == 880:
            with __import__("wave").open(str(path)) as recording:
                beginning = array.array("h", recording.readframes(recording.getframerate() // 3))
            assert max(map(abs, beginning)) == 0, "Delayed audio stream lost its starting offset"
    driver.ipc("undo")
    assert driver.ipc("snapshot")["project"]["tracks"] == before["project"]["tracks"]
    driver.ipc("redo")
    path = RESULTS / (fixture["file"] + ".svoice")
    driver.ipc("save", path=str(path))
    driver.ipc("track_remove", trackId=tracks[2]["id"])
    reopened = driver.ipc("open", path=str(path))
    assert reopened["project"]["tracks"] == tracks
    for track, frequency in zip(tracks[1:4], [440, 660, 880]):
        path = RESULTS / f"{fixture['file']}-reopened-{frequency}.mp3"
        driver.ipc("export", trackId=track["id"], format="mp3", path=str(path))
        evidence.append(analyze(path, frequency=frequency))
    return {"tracks": len(tracks), "streams": evidence, "voiceTrackPreserved": True}


def recording_with_preview(driver):
    track = driver.fresh()
    audio.defaults("b")
    driver.ipc("import_paths", paths=[str(FIXTURES / "h264-aac-short-gop.mp4")], position=0)
    driver.wait("return document.querySelector('video')?.readyState>=2;")
    driver.ipc("audio_preferences_patch", inputDevice="lab_microphone_b", outputDevice="")
    time.sleep(2.5)
    driver.click(".record-button")
    driver.wait("return !!document.querySelector('.recording-badge');")
    routing = {
        key: json.loads(audio.pactl("--format=json", "list", key))
        for key in ["source-outputs", "clients", "sources"]
    }
    (RESULTS / "recording-routing.json").write_text(json.dumps(routing, indent=2) + "\n")
    observe_frames(driver)
    time.sleep(1)
    output = capture("lab_output_b.monitor")
    assert output["peak"] > 0.001, output
    geometry = driver.js(
        "const ruler=document.querySelector('.tl-ruler'),box=ruler.getBoundingClientRect();return {left:box.left,top:box.top,width:box.width,maximum:Number(ruler.getAttribute('aria-valuemax'))};"
    )
    before_seek = driver.js(
        "return Number(document.querySelector('.tl-ruler').getAttribute('aria-valuenow'));"
    )
    seek_started = time.monotonic()
    driver.pointer(
        geometry["left"] + 8 / geometry["maximum"] * geometry["width"], geometry["top"] + 8
    )
    position = driver.js(
        "return Number(document.querySelector('.tl-ruler').getAttribute('aria-valuenow'));"
    )
    seek_seconds = time.monotonic() - seek_started
    assert -0.1 <= position - before_seek <= seek_seconds + 0.35, {
        "before": before_seek,
        "after": position,
        "elapsed": seek_seconds,
        "error": "Seeking changed the recording position",
    }
    frames = driver.js("window.__labFrames.stop();return window.__labFrames.count;")
    assert frames >= 10, frames
    driver.click(".record-button")
    driver.wait("return !document.querySelector('.recording-badge');")
    path = RESULTS / "recording-with-video.wav"
    driver.ipc("export", trackId=track, format="wav", path=str(path))
    return {
        "recording": analyze(path, frequency=660),
        "background": output,
        "frames": frames,
        "recordingPosition": position,
    }


def unavailable_codec(driver):
    driver.fresh()
    snapshot = driver.ipc(
        "import_paths", paths=[str(FIXTURES / "h264-aac-short-gop.mp4")], position=0
    )
    driver.wait("return !!document.querySelector('video')?.error;")
    message = driver.wait(
        "return document.querySelector('.toast.error .notice-content')?.textContent;"
    )
    assert "H.264" in message and "MP4" in message, message
    path = RESULTS / "audio-without-video-decoder.wav"
    driver.ipc(
        "export", trackId=snapshot["project"]["tracks"][0]["id"], format="wav", path=str(path)
    )
    return {"message": message, "audio": analyze(path, 12, 440)}


def editing(driver):
    track = driver.fresh()
    snapshot = driver.ipc(
        "import_paths", paths=[str(FIXTURES / "tone-mono-44100.wav")], trackId=track, position=0
    )
    original = snapshot["project"]["tracks"]
    clip = original[0]["clips"][0]
    driver.ipc("clip_split", trackId=track, clipId=clip["id"], position=3)
    clips = driver.ipc("snapshot")["project"]["tracks"][0]["clips"]
    assert len(clips) == 2 and abs(clips[0]["duration"] - 3) < 0.001
    driver.ipc("clip_edit", trackId=track, clipId=clips[1]["id"], mode="move", delta=2)
    path = RESULTS / "edited.wav"
    driver.ipc("export", trackId=track, format="wav", path=str(path))
    evidence = analyze(path, 10, 440)
    added = driver.ipc("track_add")["project"]["tracks"][-1]["id"]
    driver.ipc("track_remove", trackId=added)
    driver.ipc("undo")
    assert len(driver.ipc("snapshot")["project"]["tracks"]) == 2
    driver.ipc("redo")
    assert len(driver.ipc("snapshot")["project"]["tracks"]) == 1
    driver.ipc("track_patch", trackId=track, patch={"locked": True})
    try:
        driver.ipc("clip_remove", trackId=track, clipId=clips[0]["id"])
    except RuntimeError:
        pass
    else:
        raise AssertionError("Locked clip removal succeeded")
    driver.ipc("track_patch", trackId=track, patch={"locked": False})
    driver.ipc("clip_remove", trackId=track, clipId=clips[0]["id"])
    assert len(driver.ipc("snapshot")["project"]["tracks"][0]["clips"]) == 1
    return evidence


def effects(driver):
    track = driver.fresh()
    driver.ipc(
        "import_paths", paths=[str(FIXTURES / "tone-mono-44100.wav")], trackId=track, position=0
    )
    driver.click(".tl-track-name")
    driver.js("document.querySelector('[data-tab=effects]')?.click();return true;")
    driver.js(
        "const button=[...document.querySelectorAll('.inspector-tabs button,.panel-tabs button,.sidebar-tabs button')].find(button=>button.textContent.includes('Effects'));button?.click();return true;"
    )
    driver.wait("return !!document.querySelector('.preset-card select');")
    neutral = RESULTS / "effects-neutral.wav"
    driver.ipc("export", trackId=track, format="wav", path=str(neutral))
    reference = analyze(neutral, 8, 440)
    driver.js(
        "const select=document.querySelector('.preset-card select');select.value='warm';select.dispatchEvent(new Event('change',{bubbles:true}));return true;"
    )
    driver.wait("return document.querySelector('.preset-card select').value==='warm';")
    snapshot = driver.ipc("snapshot")
    processed_track = snapshot["project"]["tracks"][0]
    assert not processed_track["fxBypass"] and processed_track["effects"]["lowMid"] == 2
    processed = RESULTS / "effects-warm.wav"
    driver.ipc("export", trackId=track, format="wav", path=str(processed))
    result = analyze(processed, 8, 440)
    assert abs(result["rms"] - reference["rms"]) > reference["rms"] * 0.1
    settings = dict(processed_track["effects"], makeup=1.25, normalize=False)
    driver.ipc("track_patch", trackId=track, patch={"effects": settings})
    driver.wait("return document.querySelector('.preset-card select').value==='custom';")
    saved = driver.ipc("preset_save", name="Procedural custom", effects=settings, fxBypass=False)
    preset = next(
        preset
        for preset in saved["config"]["effectPresets"]
        if preset["name"] == "Procedural custom"
    )
    driver.js(
        "const select=document.querySelector('.preset-card select');select.value='neutral';select.dispatchEvent(new Event('change',{bubbles:true}));return true;"
    )
    driver.wait("return document.querySelector('.preset-card select').value==='neutral';")
    driver.js(
        "const select=document.querySelector('.preset-card select');select.value="
        + json.dumps(preset["id"])
        + ";select.dispatchEvent(new Event('change',{bubbles:true}));return true;"
    )
    driver.wait(
        "return document.querySelector('.preset-card select').value==="
        + json.dumps(preset["id"])
        + ";"
    )
    assert driver.ipc("snapshot")["project"]["tracks"][0]["effects"] == settings
    return {"neutral": reference, "warm": result, "customPreset": preset["name"]}


def invalid_import(driver):
    track = driver.fresh()
    try:
        driver.ipc("import_paths", paths=[str(FIXTURES / "invalid.mp3")], trackId=track, position=0)
    except RuntimeError as error:
        snapshot = driver.ipc("snapshot")
        assert not snapshot["project"]["assets"]
        return {"rejected": str(error), "applicationResponsive": True}
    raise AssertionError("Invalid MP3 was accepted")


def environment():
    values = {
        "kernel": subprocess.check_output(["uname", "-r"], text=True).strip(),
        "osRelease": Path("/etc/os-release").read_text(),
        "glibc": subprocess.check_output(["ldd", "--version"], text=True).splitlines()[0],
        "audioServer": audio.pactl("info"),
        "display": os.environ.get("GDK_BACKEND"),
        "bridge": os.environ.get("LAB_BRIDGE", "virtual"),
    }
    if Path("/usr/bin/dpkg-query").exists():
        packages = ["dpkg-query", "--show", "--showformat=${Package} ${Version}\n"]
    elif Path("/usr/bin/rpm").exists():
        packages = ["rpm", "--query", "--all", "--queryformat", "%{NAME} %{VERSION}-%{RELEASE}\n"]
    else:
        packages = ["pacman", "--query"]
    values["packages"] = subprocess.check_output(packages, text=True).splitlines()
    try:
        with urllib.request.urlopen("http://127.0.0.1:5174/api/health") as response:
            health = json.load(response)
        values["application"] = health
        mappings = Path(f"/proc/{health['pid']}/maps").read_text().splitlines()
        values["loadedLibraries"] = sorted(
            {
                line.split()[-1]
                for line in mappings
                if "/" in line
                and any(
                    name in line
                    for name in ["libwebkit", "libjavascriptcore", "libgst", "libpulse", "libc.so"]
                )
            }
        )
    except (OSError, urllib.error.URLError):
        pass
    return values


def main():
    RESULTS.mkdir(parents=True, exist_ok=True)
    driver = Driver()
    report = {
        "schema": 1,
        "preset": os.environ.get("LAB_PROFILE", "vm"),
        "tests": [],
        "environment": None,
        "automation": "Native WebKitGTK WebDriver; real Tauri IPC and real desktop audio/video; no mocked application APIs",
        "selection": sys.argv[sys.argv.index("--match") + 1] if "--match" in sys.argv else None,
        "extended": "--extended" in sys.argv,
    }
    extended = "--extended" in sys.argv
    match = re.compile(sys.argv[sys.argv.index("--match") + 1]) if "--match" in sys.argv else None

    def case(name, action, required=True):
        if match and name != "desktop.start" and not match.search(name):
            return True
        started = time.monotonic()
        record = {"name": name, "status": "passed", "required": required}
        try:
            record["evidence"] = action()
        except Exception as error:
            record.update(
                status="failed" if required else "skipped",
                error=str(error),
                traceback=traceback.format_exc(),
            )
            driver.screenshot("failure-" + str(len(report["tests"])))
        record["seconds"] = round(time.monotonic() - started, 3)
        report["tests"].append(record)
        print(f"{record['status'].upper():8} {name} ({record['seconds']}s)", flush=True)
        (RESULTS / "report.json").write_text(
            json.dumps(report, ensure_ascii=False, indent=2) + "\n"
        )
        return record["status"] == "passed"

    try:
        if case("desktop.start", driver.start):
            report["environment"] = environment()
            case("audio.two-independent-inputs-and-outputs", lambda: catalog(driver))
            manifest = json.loads((FIXTURES / "manifest.json").read_text())
            report["fixtures"] = {
                "origin": manifest["origin"],
                "sha256": __import__("hashlib")
                .sha256((FIXTURES / "manifest.json").read_bytes())
                .hexdigest(),
            }
            for fixture in manifest["files"]:
                if fixture["kind"] == "audio":
                    case(
                        "audio.import-and-export." + fixture["file"],
                        lambda fixture=fixture: imports_and_exports(driver, fixture),
                    )
            case("audio.mix-export-and-project-roundtrip", lambda: mixed_project(driver))
            case("audio.effects-preset-custom-and-export", lambda: effects(driver))
            case("timeline.split-move-remove-lock-undo-redo", lambda: editing(driver))
            case("audio.system-default-pinned-unplug-replug", lambda: routing(driver))
            case("audio.microphone-a-record-and-export", lambda: microphone(driver, "a"))
            case("audio.microphone-b-record-and-export", lambda: microphone(driver, "b"))
            case("audio.microphone-system-default-live-switch", lambda: microphone_switch(driver))
            for fixture in manifest["files"]:
                if fixture["kind"] == "multistream":
                    case(
                        "video.multistream-and-project-roundtrip." + fixture["file"],
                        lambda fixture=fixture: multistream(driver, fixture),
                    )
                if fixture["kind"] in ["video", "video-only"] and (fixture["required"] or extended):
                    case(
                        "video.import-preview-and-audio." + fixture["file"],
                        lambda fixture=fixture: video(driver, fixture),
                        fixture["required"]
                        and not (
                            os.environ.get("LAB_CODECS") == "minimal"
                            and fixture["file"].startswith(("h264", "video-without"))
                        ),
                    )
            available_h264 = os.environ.get("LAB_CODECS") != "minimal"
            case("video.rapid-scrubbing-and-clock", lambda: scrubbing(driver), available_h264)
            case(
                "video.1080p-scrubbing-and-clock",
                lambda: scrubbing(driver, "h264-1080p-long-gop.mp4"),
                available_h264,
            )
            case(
                "video.recording-with-preview",
                lambda: recording_with_preview(driver),
                available_h264,
            )
            if not available_h264:
                case(
                    "video.missing-codec-message-and-audio-export",
                    lambda: unavailable_codec(driver),
                )
            case("video.double-click-fullscreen-and-escape", lambda: fullscreen(driver))
            case("audio.invalid-import-rejected", lambda: invalid_import(driver))
            driver.screenshot("final-desktop")
    finally:
        driver.close()
        report["environment"] = report["environment"] or environment()
        (RESULTS / "report.json").write_text(
            json.dumps(report, ensure_ascii=False, indent=2) + "\n"
        )
        tests = report["tests"]
        suite = ET.Element(
            "testsuite",
            name=report["preset"],
            tests=str(len(tests)),
            failures=str(sum(item["status"] == "failed" for item in tests)),
            skipped=str(sum(item["status"] == "skipped" for item in tests)),
            time=str(sum(item["seconds"] for item in tests)),
        )
        for record in tests:
            item = ET.SubElement(
                suite,
                "testcase",
                name=record["name"],
                classname="linux.compatibility",
                time=str(record["seconds"]),
            )
            if record["status"] in ["failed", "skipped"]:
                ET.SubElement(
                    item,
                    "failure" if record["status"] == "failed" else "skipped",
                    message=record["error"],
                ).text = record["traceback"]
            ET.SubElement(item, "system-out").text = json.dumps(
                record.get("evidence", {}), ensure_ascii=False
            )
        ET.ElementTree(suite).write(RESULTS / "junit.xml", encoding="utf-8", xml_declaration=True)
    raise SystemExit(1 if any(item["status"] == "failed" for item in report["tests"]) else 0)


def catalog(driver):
    result = driver.ipc("audio_devices")["audioDevices"]
    assert result["available"] and result["nativeRouting"]
    assert {item["id"] for item in result["outputs"]} == {"lab_output_a", "lab_output_b"}, result
    assert {"lab_microphone_a", "lab_microphone_b"}.issubset(
        {item["id"] for item in result["inputs"]}
    ), result
    return result


if __name__ == "__main__":
    main()
