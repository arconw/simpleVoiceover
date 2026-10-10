# Verification

Observed on 2026-10-10 against the simpleVoiceover 1.3.0 release artifacts in `build/`. All 13 configurations completed the full extended suite against the Linux artifact hashes: **476 checks passed, 19 optional checks were skipped, and no required check failed**. Comparison with the preserved initial results reports **zero regressions and 23 improvements**.

Generated JSON/JUnit reports, package inventories, loaded libraries, artifact/image hashes, screenshots, exported media, and clock/frame observations are available in `build/linux-testbench/`. Open `build/linux-testbench/index.html` for the report index; `comparison.json` contains the regression comparison. `build/checksums.sha256` identifies the four local release artifacts.

## Observed configurations

| Configuration                                    | glibc | WebKit         | Audio      | Passed / skipped |
| ------------------------------------------------ | ----- | -------------- | ---------- | ---------------- |
| Ubuntu 22.04 DEB                                 | 2.35  | 2.50.4         | PulseAudio | 38 / 0           |
| Ubuntu 24.04 DEB                                 | 2.39  | 2.52.6         | PipeWire   | 38 / 0           |
| Debian 12 DEB                                    | 2.36  | 2.50.6         | PulseAudio | 38 / 0           |
| Debian 13 DEB                                    | 2.41  | 2.54.0         | PipeWire   | 38 / 0           |
| Fedora 43 RPM                                    | 2.42  | 2.52.5         | PipeWire   | 37 / 1           |
| Fedora 44 RPM                                    | 2.43  | 2.54.1         | PipeWire   | 37 / 1           |
| Arch rolling AppImage                            | 2.44  | Bundled 2.50.4 | PipeWire   | 38 / 0           |
| Ubuntu 22.04 AppImage                            | 2.35  | Bundled 2.50.4 | PulseAudio | 38 / 0           |
| Fedora 44 AppImage ELF with system libraries     | 2.43  | 2.54.1         | PipeWire   | 37 / 1           |
| Ubuntu 24.04 DEB, nested Wayland                 | 2.39  | 2.52.6         | PipeWire   | 38 / 0           |
| Ubuntu 24.04 AppImage ELF, minimal system codecs | 2.39  | 2.52.6         | PipeWire   | 23 / 16          |
| QEMU Ubuntu 22.04 DEB, GA kernel                 | 2.35  | 2.50.4         | PulseAudio | 38 / 0           |
| QEMU Ubuntu 22.04 DEB, HWE kernel                | 2.35  | 2.50.4         | PulseAudio | 38 / 0           |

Containers shared host kernel `7.2.9-200.fc44.x86_64`. The actual QEMU guest kernels were `5.15.0-198-generic` and `6.8.0-138-generic`, with four vCPUs and 6 GiB RAM each. Both guests shut down after their checks. The Ubuntu 24.04 GA VM profile is prepared but has not been booted or verified.

AppImage bundled mode runs its unchanged extracted `AppRun`, avoiding a container FUSE requirement. System-library mode runs the extracted application ELF separately; it does not verify the full AppImage launcher. The Wayland configuration uses nested Weston with a virtual display and does not claim full GNOME/KDE portal or hardware GPU coverage. Fedora 43's WebDriver lacks mouse commands, so its checks use actual pointer input through the local virtual VNC display; `pointer.json` records this fallback.

## Functionality exercised

- Import and export WAV/MP3, including mono/stereo, 16/24-bit/float WAV, 44.1/48/96 kHz input, CBR/VBR MP3, and Unicode filenames. Exports are decoded and checked for finite, non-silent samples and the expected tone.
- Apply presets through the real dropdown, verify processed export, modify a preset into Custom, save it, and apply it again.
- Split, move, remove, lock, add/delete tracks, Undo/Redo, mix export, and project save/reopen.
- Change System Default outputs during playback, retain an explicit output, simulate unplug/replug, record independently from two distinct microphones, and change the system default microphone during an active recording. Tone checks distinguish microphone A at 440 Hz from B at 660 Hz.
- Import H.264 MP4/MKV with AAC-LC, MP3, FLAC, integer PCM and float PCM audio. Three-stream MP4/MKV fixtures verify stream order, retained starting delays, the armed voice track below the imported streams, initial mute state, independent WAV/MP3 exports, Undo/Redo, and portable project reopening.
- Play video without audio, retain existing MOV/VP8/VP9 WebM support, exercise variable frame rate and short/long GOPs, record while previewing, reject seeking during recording, and enter/exit native fullscreen without losing application responsiveness.
- Repeatedly seek ordinary and 1080p long-GOP video with real pointer input. Accepted runs delivered **18.5–30.5 presented frames/s**, with maximum final frame/timeline disagreement **0.297 s**, no unexpected backward playhead jumps, and no required clock/FPS failure. Acceptance requires at least 10 presented frames/s and less than 0.35 s disagreement. These are virtual-display measurements, not a performance guarantee for every file, GPU, concurrent workload, or hardware setup.
- Reject invalid/unsupported imports without changing the previous project, dirty state, or owned media files.

All fixtures are original procedural media; no Downloads file or other user media is used.

## Fixes verified

RPM installation now uses valid Fedora package dependency names and includes the separate GStreamer OpenH264 plugin. The package installs normally with dependencies; checks do not bypass them. A Fedora 44 package transaction also confirmed that installation does not replace its existing PipeWire server.

Matroska audio streams now decode independently, including large PCM packets. Videos without audio import successfully. Audio/video start and seek preparation wait for a ready target frame, obsolete seek requests are canceled, and decoder compensation is bounded. Linux MP4/MOV and Matroska/WebM preview responses hide audio track metadata while preserving byte positions and the original source, so WebKit does not run another audio playback clock. Windows preview responses retain the original bytes. HTTP range and saved-source tests cover this distinction on Linux. The project owner additionally reported successful manual checks on Windows and an older Ubuntu release; their exact OS versions and scenario coverage were not recorded.

Earlier candidates exposed expensive corrective video seeks and a test-driver race between Pause and the next recording scenario. The final reports were regenerated after those fixes; partial or older-artifact runs cannot pass the regression comparison. Diagnostic observations remain under `build/linux-testbench/setup/`.

## Build requirements and limits

Linux artifacts were built with Ubuntu 22.04 / glibc 2.35, WebKit 2.50.4 and GStreamer 1.20.3. The application ELF's highest required GLIBC symbol is `GLIBC_2.34`; this does not lower the documented glibc 2.35 package/runtime baseline. Ubuntu 26.04 LTS is recommended for DEB and Fedora 44 for RPM, while dependency declarations retain the older baseline. Older systems are used at the user's own risk. These recommendations are separate from the exact runtime configurations above: Ubuntu 26.04 and Fedora 45 Beta were not exercised by this bench.

The three Fedora configurations skip exploratory HEVC preview because their exercised official codec set does not decode it. The minimal-codec configuration skips 16 optional preview scenarios; its missing-codec message and audio export remain required and passed. H.264 MP4/MKV is the primary video scope. AC-3, DTS, and Opus audio are outside the current decoding scope; a container extension alone does not imply support for every embedded codec.

Project validation passed: **60 TypeScript tests, 51 Rust tests**, two explicitly ignored Rust tests, frontend build, frontend formatting, and Rust formatting. The bench's **eight self-tests**, Python lint/format checks, JSON/Markdown formatting, and Bash syntax checks passed. All four x86_64 artifacts were rebuilt/replaced in `build/` for release v1.3.0. The Windows EXE was cross-built successfully and checked manually by the project owner; Windows was not part of this automated Linux loop.

Automatic runs use private virtual devices and internal output capture, so they are silent on host speakers. The previously exercised manual bridge creates two independent playback streams and two capture streams connected to current host defaults. Physical speaker output, microphone quality, Bluetooth wake-up/reconnection, and USB unplugging require listening and interacting with real devices in manual mode. Start a fresh manual session to exercise the current artifact; an already running session keeps its older executable until restarted.
