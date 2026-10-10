# Linux compatibility test bench

An isolated compatibility lab for the existing x86_64 simpleVoiceover release. All bench source lives in this directory. It installs artifacts from `build/`; it does not rebuild, edit, or commit the application.

Generated media, session workspaces, logs, screenshots, and reports live in `build/linux-testbench/`. VM disks and automation tools live in `build/.cache/linux-testbench/`. Rootless Podman stores container images in its normal image store. Everything created by the bench has a dedicated name or label and a scoped cleanup command.

## Quick start

Run from the project root:

```bash
linux-testbench/lab doctor
linux-testbench/lab list
linux-testbench/lab prepare ubuntu22-pulse
linux-testbench/lab fixtures
linux-testbench/lab manual ubuntu22-pulse
```

Open <http://127.0.0.1:6080/vnc.html?autoconnect=1&resize=scale>. The desktop contains the actual release application and a terminal with audio controls. Import the original procedural files from `/fixtures`; save projects and exports in `/results`.

Automatic checks use the same image, application, display, audio server, and device setup:

```bash
linux-testbench/lab auto ubuntu22-pulse
linux-testbench/lab auto ubuntu24-pipewire debian12-pulse
linux-testbench/lab prepare --all
linux-testbench/lab auto --all --extended
linux-testbench/lab report
```

Exit status `1` means at least one required check failed. A successful desktop launch or a moving playhead alone cannot make an audio/video check pass.

## Presets and coverage

| Preset                   | Userspace    | Private audio server    | Release      | WebKit     |
| ------------------------ | ------------ | ----------------------- | ------------ | ---------- |
| `ubuntu22-pulse`         | Ubuntu 22.04 | PulseAudio              | DEB          | System 4.1 |
| `ubuntu24-pipewire`      | Ubuntu 24.04 | PipeWire + Pulse server | DEB          | System 4.1 |
| `debian12-pulse`         | Debian 12    | PulseAudio              | DEB          | System 4.1 |
| `debian13-pipewire`      | Debian 13    | PipeWire + Pulse server | DEB          | System 4.1 |
| `fedora43-pipewire`      | Fedora 43    | PipeWire + Pulse server | RPM          | System 4.1 |
| `fedora44-pipewire`      | Fedora 44    | PipeWire + Pulse server | RPM          | System 4.1 |
| `arch-pipewire`          | Arch rolling | PipeWire + Pulse server | AppImage     | Bundled    |
| `ubuntu22-appimage`      | Ubuntu 22.04 | PulseAudio              | AppImage     | Bundled    |
| `fedora44-system-webkit` | Fedora 44    | PipeWire + Pulse server | AppImage ELF | System 4.1 |

The initial matrix covers the glibc baseline, another Ubuntu LTS, both supported Debian generations, both targeted Fedora generations, and a rolling distribution. It varies one important axis at a time instead of generating every possible combination. `presets.json` is the editable catalog.

Container presets share the host kernel. A Fedora container running on an Ubuntu kernel is a Fedora userspace check; it is not evidence that the Fedora kernel, desktop session, or Bluetooth stack was tested. The kernel profiles below reuse the container matrix inside a VM rather than maintaining another copy of the test implementation.

## Constructor

Overrides work with `prepare`, `manual`, and `auto`:

```bash
linux-testbench/lab auto ubuntu24-pipewire --audio pulseaudio
linux-testbench/lab manual fedora44-pipewire --artifact appimage
linux-testbench/lab auto arch-pipewire --webkit system
linux-testbench/lab auto linux-testbench/examples/ubuntu24-minimal.json
linux-testbench/lab manual linux-testbench/examples/ubuntu24-wayland.json --port 6081
```

| Option                          | Meaning                                                                                           |
| ------------------------------- | ------------------------------------------------------------------------------------------------- |
| `--audio pulseaudio\|pipewire`  | Select a private audio server; the app uses the Pulse protocol in both cases.                     |
| `--artifact deb\|rpm\|appimage` | Select the current release; native package formats must match the distribution.                   |
| `--webkit system\|bundled`      | Bundled requires AppImage; system tests the distribution's WebKit 4.1.                            |
| `--webkit-version VERSION`      | Request an exact distribution package version for WebKit, JavaScriptCore, and WebDriver together. |
| `--codecs minimal\|full`        | Base/good GStreamer plugins, or those plus libav/bad packages available in that distribution.     |
| `--display x11\|wayland`        | Xvfb/Openbox, or nested Weston on the same virtual X display.                                     |
| `--base-image IMAGE`            | Override the base with a compatible OCI image, digest, or repository snapshot image.              |

A custom JSON preset has an `extends` field naming a built-in preset and overrides any of its fields. Its filename determines its session name. CLI overrides take precedence. Package sets are reused across compatible runtime variants; changing the package set creates another image, while changing the artifact or display creates another session.

Older WebKit packages must actually exist in the selected repositories or snapshot. Pinning one library while updating the rest is intentionally unsupported. Arch rejects an isolated `--webkit-version`; use an Arch Linux Archive repository snapshot in a custom base image. OCI tags and normal repositories move over time. For reproducible reruns, preserve the recorded image ID and full package inventory and use a repository snapshot for a rebuild. `prepare --refresh` deliberately updates the image.

AppImage bundled mode extracts the image and runs its unchanged `AppRun`, avoiding a FUSE requirement inside the container. Its WebKit uses a driver extracted from the Ubuntu 22.04 baseline image with private driver libraries; those libraries are not injected into the application. A future AppImage with a different WebKit cohort may require updating that baseline. Protocol mismatches fail visibly.

AppImage system mode copies the unchanged application ELF out of the extracted bundle so its relative RPATH cannot silently select bundled libraries. It skips `AppRun` and tests the application with system libraries. It does **not** validate the full AppImage launcher. Actual loaded libraries are recorded in each report. AppImage's bundled codecs can remain available even with `--codecs minimal`; use a system-WebKit preset when testing missing distribution codecs.

Installing a native DEB/RPM obeys its declared dependencies, which can add codecs to a minimal image. The minimal example therefore runs the extracted AppImage ELF with system WebKit. The inventory reflects the packages actually installed, rather than treating the preset name as proof of missing codecs.

Fedora's `full` set includes libav/bad and the explicit `gstreamer1-plugin-openh264` package from Fedora repositories. [Fedora documents](https://fedoraproject.org/wiki/OpenH264) that GStreamer needs this plugin separately from the OpenH264 library. RPM Fusion is not silently enabled. Unsupported HEVC or other codec combinations remain visible in the report. The bench does not claim ALSA-only or JACK support: the current application exposes native routing through the Pulse API.

## Two microphones and two outputs

Each private audio server exposes:

| ID                 | Description      | Rate            |
| ------------------ | ---------------- | --------------- |
| `lab_output_a`     | Lab Output A     | 48 kHz stereo   |
| `lab_output_b`     | Lab Output B     | 44.1 kHz stereo |
| `lab_microphone_a` | Lab Microphone A | 48 kHz mono     |
| `lab_microphone_b` | Lab Microphone B | 44.1 kHz mono   |

These are separate server devices with separate application IDs. The server also exposes ordinary output monitor sources; the app can list those as additional inputs.

Manual mode defaults to `--bridge host`. Both outputs have independent forwarding streams to the computer's default output. Both microphones receive independent capture streams from the computer's default input. No HyperX/JBL/device model is hardcoded. A one-second poll follows host default changes by moving only streams tagged with this bench's application ID. Container defaults remain independent and can be switched between A/B. Host device defaults, stream volume, and unrelated applications are not changed. This mode captures the real system microphone while the session runs; `stop` removes the bridge streams.

Automatic mode uses `virtual`: microphones generate distinct 440/660 Hz tones, outputs are captured internally, and nothing is forwarded to the computer's speakers. For a silent manual session:

```bash
linux-testbench/lab manual ubuntu22-pulse --bridge virtual
```

In the desktop terminal, use `lab-default-a`, `lab-default-b`, `lab-remove-a`, `lab-remove-b`, `lab-restore-a`, `lab-restore-b`, and `lab-devices`. Removal simulates unplugging both devices in a pair; restoration preserves their stable names. Application preferences can independently select either input/output or System Default.

The same controls are available from the host, using the container name printed at startup:

```bash
linux-testbench/lab audio CONTAINER devices
linux-testbench/lab audio CONTAINER default b
linux-testbench/lab audio CONTAINER remove b
linux-testbench/lab audio CONTAINER restore b
linux-testbench/lab shell CONTAINER
linux-testbench/lab stop CONTAINER
```

The bridge tests the private-server and host-server path. It cannot manufacture real Bluetooth reconnection, USB firmware behavior, speaker wake-up delays, or hardware clock drift. Those remain useful manual hardware checks. Containers do not receive `/dev/snd`, the host desktop bus, or your media directories.

## Media and automatic checks

`fixtures.py` generates all media from FFmpeg sine waves and `testsrc2`. No file from Downloads or other user media directories is read. The generated fixtures are offered under CC0-1.0; `manifest.json` includes codec metadata, durations, SHA256 hashes, and the generator version.

The set includes 16/24-bit and float WAV, mono/stereo, 44.1/48/96 kHz, MP3 CBR/VBR, a Unicode filename, MP4/MOV/MKV/WebM, H.264/AAC, H.264/PCM, VP8/VP9/Vorbis, short/long GOPs, variable frame rate, HEVC, video without audio, and intentionally invalid MP3 data. H.264 video without audio is a required check. HEVC remains exploratory and is enabled by `--extended`; unsupported combinations are reported as skipped with their errors, never as passed. The required MP4/Matroska set also includes three audio streams with separate tones, a delayed stream, PCM 24-bit/float and FLAC in Matroska, and a 1080p long-GOP MP4.

`checks.py` controls the real Tauri webview through native WebKitGTK WebDriver. There is no Chromium surrogate, frontend test build, injected replacement transport, or mocked application API. Checks cover:

- Actual device enumeration with two independent inputs and outputs.
- Import every audio fixture and export WAV/MP3; verify duration, codec, finite samples, non-silence, and the expected frequency.
- Mix two tones, export both formats, save/reopen a project, and verify both tones survive.
- Apply an effect through the actual preset dropdown, verify processed export, edit a preset into Custom, save it, and apply it again.
- Split/move/delete clips, protect locked clips, add/remove tracks, and check Undo/Redo.
- Import every stream from multitrack MP4/Matroska, verify track order and starting delay, export each stream in WAV/MP3, and reopen the project.
- Playback while changing the private default, pinning an output, unplugging it, falling back, and restoring it; capture actual PCM from each output monitor.
- Record each virtual microphone through the application and verify its distinct tone in the exported recording.
- Change the system default microphone during a take and verify both device tones in the saved recording.
- Import video, advance decoded frames, and capture its audio.
- Seek back and forth through the real pointer event path at 360p and 1080p; measure delivered FPS, dropped frames, video/timeline clock agreement, actual presented-frame timestamps when the webview exposes them, and backward playhead jumps. The 10 FPS floor detects severe stalls; virtual software rendering is not a hardware performance benchmark.
- Enter native fullscreen by double-clicking the preview and exit with Escape; confirm the application remains responsive.
- Reject invalid media without corrupting the project.

JSON and JUnit reports, screenshots, private service logs, exported media, application workspaces, and `configuration.json` are retained per configuration. Reports record the artifact hash, image ID, bench source hash, packages, glibc, audio server, kernel, and loaded library paths. A repeated automatic run replaces the same configuration's previous report; `--keep` leaves its container available for inspection. Stop that container before rerunning.

Some distribution WebDriver builds lack mouse interaction support. If a mouse command reports `unsupported operation`, the bench sends actual pointer input through its existing local VNC display. `pointer.json` records the fallback. Tauri IPC, audio devices, codecs, and the application remain the same.

## Fix/test regression loop

```bash
linux-testbench/lab baseline
linux-testbench/lab auto ubuntu22-appimage --match 'scrubbing|multistream|effects|timeline\.'
linux-testbench/lab auto --all --extended
linux-testbench/lab vm auto ubuntu22-generic --extended
linux-testbench/lab vm auto ubuntu22-hwe --extended
linux-testbench/lab compare
```

`baseline` preserves JSON evidence without duplicating release binaries. It refuses to replace an existing baseline. `--match` runs focused cases plus desktop startup; complete coverage must be restored before comparing. `compare` fails if a previously passing case is missing, fails, or becomes skipped, or if a new required case fails. It also rejects reports for an older artifact hash, including old VM runs. Previously known failures remain visible; they are not silently made successful.

Minimal-codec presets keep audio, editing, and multistream checks required. Preview cases whose H.264 decoder is intentionally absent report unsupported evidence as skipped. Full-codec presets require these previews to work. Runtime Windows checks are performed manually by the project owner; the Linux loop does not claim Windows runtime coverage.

## Different kernels with QEMU/KVM

The VM catalog includes Ubuntu 22.04 GA, Ubuntu 22.04 HWE, and Ubuntu 24.04 GA. GA/HWE share one verified cloud-image base and use separate qcow2 overlays. Cloud-init installs the selected kernel and reboots before tests start. Reports record the kernel actually booted, rather than assuming a version from the profile name.

```bash
linux-testbench/lab vm prepare ubuntu22-hwe
linux-testbench/lab vm auto ubuntu22-generic
linux-testbench/lab vm auto ubuntu22-hwe --preset arch-pipewire
linux-testbench/lab vm manual ubuntu24-generic
linux-testbench/lab vm stop ubuntu24-generic
```

Any built-in userspace preset can run on each guest kernel through `--preset`; choose representative combinations instead of multiplying the entire matrix. Automatic mode downloads/provisions on first use, runs the same checks in the guest, copies results to `build/linux-testbench/vm/`, and shuts down that VM. Manual mode prints a localhost browser URL forwarded over SSH and leaves the VM running.

Manual guest audio uses virtual HDA through QEMU's Pulse backend, then the guest Pulse server, then the private container server. It reaches the host microphone/speakers. Kernel checks use Ubuntu kernels; they do not claim native Fedora/Arch kernel coverage or a full GNOME/KDE portal session. Add another cloud image/profile to `presets.json` for those scenarios, with a matching provisioning package configuration. The current VM provisioner supports Ubuntu cloud images.

QEMU audio streams receive a profile-specific application ID. A host follower moves only these streams when the host default input/output changes. Both the follower and SSH browser forwarding stop with that VM.

VM defaults are four vCPUs, 6 GiB RAM, a sparse 32 GiB disk, localhost-only SSH, and key authentication. `vm prepare` creates the disk; kernel installation happens during the first `vm auto`/`vm manual` boot. First provisioning may take several minutes. Its console log is under `build/.cache/linux-testbench/vm/PROFILE/console.log`.

## Requirements and maintenance

Container runs need Linux x86_64, Python 3.10+, rootless Podman with subordinate UID/GID mappings, network access for package setup, and built release artifacts matching `package.json`. The app's release build baseline remains Ubuntu 22.04/glibc 2.35; these runtime checks do not change release dependencies. Manual host bridging needs the host Pulse-compatible socket, supplied by PulseAudio or PipeWire's Pulse server.

VM runs additionally need Python 3.11+, KVM access, `qemu-system-x86_64`, `qemu-img`, `genisoimage`, `curl`, and OpenSSH tools. Runtime containers use the distribution Python version, including Python 3.10 in Ubuntu 22.04. The host VM downloader uses Python 3.11's streaming file hash.

Headless containers have no logind seat. For WirePlumber 0.4, the bench disables Bluetooth seat monitoring in the private user's configuration, while retaining normal routing/default policy. This avoids failing startup before any virtual devices can be tested. Actual Bluetooth policy stays in the host audio server and requires manual hardware testing.

```bash
python3 -B -m unittest discover -s linux-testbench -p 'test_*.py'
ruff check linux-testbench
ruff format --check linux-testbench
npx prettier --check linux-testbench/presets.json linux-testbench/examples/*.json linux-testbench/README.md
```

No application tests, configuration, packaging, or CI files depend on this prototype. To remove it:

```bash
linux-testbench/lab clean
```

Then delete `linux-testbench/`. Cleanup stops only this bench's labeled containers and named VMs, removes its images, generated fixtures, reports, cached tools, and VM disks, and preserves application releases. OCI base layers shared with other tools may remain in Podman's store. Cleanup never performs a global container/image prune.

Until observed runtime checks pass, a prepared image or configured profile is only available test coverage, not verified compatibility. Consult `build/linux-testbench/index.html` for actual results and inspect failed checks before declaring a release compatible.

[The 1.3.0 verification record](VERIFICATION.md) documents the completed full runs, package and media fixes, measured playback/seek behavior, and remaining optional codec limits. The automatic suite passed on all 13 exercised configurations. Recommended release distributions and these observed configurations are listed separately; a preset alone is not a compatibility guarantee.
