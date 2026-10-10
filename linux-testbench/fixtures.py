import hashlib
import json
import subprocess
from pathlib import Path


def generate(directory):
    directory = Path(directory)
    directory.mkdir(parents=True, exist_ok=True)
    records = []
    audio = [
        ("tone-mono-44100.wav", "pcm_s16le", 44100, 1, 440, []),
        ("tone-stereo-48000-24bit.wav", "pcm_s24le", 48000, 2, 660, []),
        ("tone-stereo-48000-float.wav", "pcm_f32le", 48000, 2, 880, []),
        ("tone-stereo-96000.wav", "pcm_s16le", 96000, 2, 550, []),
        ("tone-cbr.mp3", "libmp3lame", 44100, 2, 440, ["-b:a", "128k"]),
        ("tone-vbr.mp3", "libmp3lame", 48000, 2, 660, ["-q:a", "3"]),
        ("Тест микрофона.wav", "pcm_s16le", 48000, 1, 440, []),
    ]
    for name, codec, rate, channels, frequency, extra in audio:
        subprocess.run(
            [
                "ffmpeg",
                "-nostdin",
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                f"sine=frequency={frequency}:sample_rate={rate}:duration=8",
                "-ac",
                str(channels),
                "-c:a",
                codec,
                *extra,
                str(directory / name),
            ],
            check=True,
        )
        records.append(
            {"file": name, "kind": "audio", "frequency": frequency, "duration": 8, "required": True}
        )
    videos = [
        ("h264-aac-short-gop.mp4", "libx264", "aac", ["-g", "15", "-movflags", "+faststart"], True),
        ("h264-aac-long-gop.mp4", "libx264", "aac", ["-g", "240", "-movflags", "+faststart"], True),
        (
            "h264-1080p-long-gop.mp4",
            "libx264",
            "aac",
            ["-g", "240", "-preset", "ultrafast", "-movflags", "+faststart"],
            True,
        ),
        ("h264-aac.mov", "libx264", "aac", ["-g", "30"], True),
        ("h264-pcm.mkv", "libx264", "pcm_s16le", ["-g", "30"], True),
        ("h264-aac.mkv", "libx264", "aac", ["-g", "30"], True),
        ("h264-mp3.mkv", "libx264", "libmp3lame", ["-g", "30"], True),
        ("h264-flac.mkv", "libx264", "flac", ["-g", "30"], True),
        ("h264-pcm24.mkv", "libx264", "pcm_s24le", ["-g", "30"], True),
        ("h264-float.mkv", "libx264", "pcm_f32le", ["-g", "30"], True),
        (
            "vp8-vorbis.webm",
            "libvpx",
            "libvorbis",
            ["-deadline", "realtime", "-cpu-used", "8"],
            True,
        ),
        (
            "vp9-vorbis.webm",
            "libvpx-vp9",
            "libvorbis",
            ["-deadline", "realtime", "-cpu-used", "8"],
            True,
        ),
        (
            "h264-variable-fps.mp4",
            "libx264",
            "aac",
            ["-vf", "select='not(mod(n,3))+not(mod(n,5))'", "-vsync", "vfr", "-g", "30"],
            True,
        ),
        (
            "hevc-aac.mp4",
            "libx265",
            "aac",
            ["-x265-params", "pools=1:frame-threads=1:log-level=error", "-tag:v", "hvc1"],
            False,
        ),
    ]
    for name, video_codec, audio_codec, extra, required in videos:
        subprocess.run(
            [
                "ffmpeg",
                "-nostdin",
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc2=size="
                + ("1920x1080" if "1080p" in name else "640x360")
                + ":rate=30:duration=12",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:sample_rate=48000:duration=12",
                "-threads",
                "2",
                "-c:v",
                video_codec,
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                audio_codec,
                "-ac",
                "2",
                *extra,
                "-shortest",
                str(directory / name),
            ],
            check=True,
        )
        records.append(
            {"file": name, "kind": "video", "frequency": 440, "duration": 12, "required": required}
        )
    for extension, codec in [("mkv", "pcm_s16le"), ("mp4", "aac")]:
        name = "h264-three-audio-streams." + extension
        arguments = [
            "ffmpeg",
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=640x360:rate=30:duration=12",
        ]
        for frequency in [440, 660, 880]:
            if frequency == 880:
                arguments.extend(["-itsoffset", "0.5"])
            arguments.extend(
                ["-f", "lavfi", "-i", f"sine=frequency={frequency}:sample_rate=48000:duration=11.5"]
            )
        arguments.extend(
            [
                "-map",
                "0:v:0",
                "-map",
                "1:a:0",
                "-map",
                "2:a:0",
                "-map",
                "3:a:0",
                "-c:v",
                "libx264",
                "-threads",
                "2",
                "-pix_fmt",
                "yuv420p",
                "-g",
                "30",
                "-c:a",
                codec,
                "-ac",
                "2",
                "-metadata:s:a:0",
                "language=eng",
                "-metadata:s:a:1",
                "language=rus",
                "-metadata:s:a:2",
                "language=deu",
                str(directory / name),
            ]
        )
        subprocess.run(arguments, check=True)
        records.append({"file": name, "kind": "multistream", "duration": 12, "required": True})
    subprocess.run(
        [
            "ffmpeg",
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=640x360:rate=30:duration=8",
            "-c:v",
            "libx264",
            "-threads",
            "2",
            "-pix_fmt",
            "yuv420p",
            "-an",
            str(directory / "video-without-audio.mp4"),
        ],
        check=True,
    )
    records.append(
        {"file": "video-without-audio.mp4", "kind": "video-only", "duration": 8, "required": True}
    )
    (directory / "invalid.mp3").write_bytes(b"procedural-invalid-audio\x00\xff")
    records.append({"file": "invalid.mp3", "kind": "invalid", "required": True})
    for record in records:
        record["sha256"] = hashlib.sha256((directory / record["file"]).read_bytes()).hexdigest()
        if record["kind"] != "invalid":
            metadata = subprocess.check_output(
                [
                    "ffprobe",
                    "-v",
                    "error",
                    "-show_streams",
                    "-show_format",
                    "-of",
                    "json",
                    str(directory / record["file"]),
                ],
                text=True,
            )
            record["metadata"] = json.loads(metadata)
    version = subprocess.check_output(["ffmpeg", "-version"], text=True).splitlines()[0]
    manifest = {
        "schema": 1,
        "origin": "Procedural FFmpeg testsrc2 patterns and sine tones; no external media",
        "license": "CC0-1.0",
        "generator": version,
        "files": records,
    }
    (directory / "manifest.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n"
    )
    print(f"Generated {len(records)} procedural fixtures in {directory}", flush=True)


if __name__ == "__main__":
    import sys

    generate(sys.argv[1])
