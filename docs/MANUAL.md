# simpleVoiceover manual

Back to the [project overview](../README.md).

A voiceover studio for Windows and Linux, version 1.3.0, built with Tauri, React, and Rust. Windows uses WebView2; Linux uses WebKitGTK 4.1. The app window owns the engine lifecycle; no separate console, FFmpeg, or external media service is required.

## Run and build

`svoice start` selects the newest versioned Windows executable in `build/`.

```bash
svoice start
svoice status
svoice logs
svoice stop
svoice help
```

`stop` requests a normal window close. Unsaved changes require saving, discarding, or canceling in the app. Recording and ongoing file operations prevent closing. Closing the window shuts down the embedded HTTP server and audio channel.

Build Windows from WSL:

```bash
bin/build-windows
```

Requires Node.js, Rust, the `x86_64-pc-windows-msvc` target, cargo-xwin, and the LLVM resource compiler `llvm-rc`. The script uses an installed LLVM or the local `.build-tools/llvm` toolchain. Windows requires WebView2 Runtime. The build script writes a versioned executable in `build/` and replaces repeated builds of the same version.

Build a Debian package on Ubuntu:

```bash
sudo apt install build-essential pkg-config libwebkit2gtk-4.1-dev librsvg2-dev libpulse-dev libgstreamer1.0-dev
bin/build-deb
```

The package is generated in `build/.cache/release/bundle/deb` and copied to `build/`. Install it with `sudo apt install ./build/simpleVoiceover_1.3.0_amd64.deb`, then launch `simpleVoiceover`. Runtime dependencies include WebKitGTK 4.1 version 2.40 or newer, GTK 3, desktop file chooser portals, the PulseAudio client library, and GStreamer media plugins. Playback and recording use an active PulseAudio-compatible server, including PipeWire's PulseAudio implementation. File dialogs use the XDG desktop portal on Linux. WSL requires WSLg for a visible window and its PulseAudio bridge for sound. This package inherits the minimum library versions of the system used to build it; release builds use Ubuntu 22.04 / glibc 2.35.

Build all Linux formats with `bin/build-linux`, an RPM with `bin/build-rpm`, or an AppImage with `bin/build-appimage`. All outputs are copied to `build/`, with Cargo intermediates under `build/.cache/`. Obsolete builds and backup copies are removed; `build/` is excluded from Git. Ubuntu 26.04 LTS is recommended for DEB and Fedora 44 for RPM. These recommendations do not prevent installation on older distributions that satisfy the package dependencies; those systems are used at the user's own risk. DEB accepts both GTK package names, `libgtk-3-0` and `libgtk-3-0t64`; RPM uses Fedora dependency names. AppImage requires glibc 2.35 or newer and bundles WebKitGTK and its media framework. All Linux formats require a graphical session and an active PulseAudio-compatible server. Verified runtime configurations and codec limits are recorded in [the Linux test bench verification](../linux-testbench/VERIFICATION.md). Fedora H.264 preview support requires `gstreamer1-plugin-openh264` from the `fedora-cisco-openh264` repository; the RPM declares this dependency. Fedora 45 is currently Beta and has not been verified.

Version 1.3.0 is available as EXE, AppImage, DEB and RPM from [GitHub Releases](https://github.com/arconw/simpleVoiceover/releases/tag/v1.3.0), with SHA-256 checksums. Its Linux artifacts use a glibc 2.35 baseline; the older v1.2.1 DEB retains its original glibc 2.43 requirement. Each release states recommendations, build requirements and verified runtime distributions separately. Application versions stay synchronized across npm, Cargo, their lockfiles, and Tauri; authorized release commits receive annotated `v<version>` tags. Building does not create a commit or tag. The standalone [Linux test bench](../linux-testbench/README.md) is included as deployable source; generated fixtures, VM disks, reports and binaries are excluded from Git.

## Editing

The menu on the left contains import, open, save, working directory, export, settings, and help. The central toolbar above the tracks contains editing tools, playback and recording controls, preview details, the timer, microphone level, and zoom. Preview labels and filenames truncate to the available width; the extension stays visible, and hover reveals the full name.

The window has no system title bar. Minimize, maximize, and close controls are in the upper right corner. Close retains recording and unsaved-change protection. Drag the window by its video area; the cursor indicates dragging. Context menus and ordinary text selection are disabled; input text can still be selected.

Double-click the video to toggle fullscreen preview. Escape exits fullscreen. Window dragging starts after pointer movement, so a stationary double-click does not move the window.

Each file has two buttons. Add places a clip on the selected track at the playhead. Remove removes the asset and all its clips from the project. Undo restores removal; unlock any track using the asset before removing it. The original source on disk is retained.

Clip movement and trimming use a local preview and commit once on release. The preview remains in place until the engine confirms the edit, avoiding a jump back to the old position. Operations show a loader near the top of the video only after 300 ms; quick edits do not flash operation messages or controls.

The magnet button next to the split tool toggles snapping without changing the active cursor tool. When enabled, moving clips align with nearby clip edges. Each track has a trash button; removal requires a localized confirmation and can be undone.

Import through a native file dialog or drag and drop. Rust reads local disks directly. The source-file limit is 100,000,000,000 bytes. Import combines copying and decoding: repeated reads and backward seeks use already copied regions, and skipped video bytes are copied before completion. Seeking to an MP4 index at the end does not copy the preceding file first. Import and saving report progress. Rust owns decoding, waveform generation, effects, editing, and export. Large media is streamed; decoded PCM can exceed the source size.

Audio import supports AAC-LC, MP3, WAV, FLAC, ALAC, and Vorbis. The primary video scope is H.264 in MP4 and Matroska (MKV). Matroska audio is checked with AAC-LC, MP3, FLAC, integer PCM, and floating-point PCM. A container extension does not guarantee support for every codec inside it; AC-3, DTS, and Opus audio are outside the current scope. Existing MOV and WebM support remains available.

Matroska imports create a video track and an independent audio track for each audio stream, above the recording track. MP4 files with multiple audio streams use the same layout. The first stream is audible initially; additional streams start muted and can be selected with Mute/Solo. Stream names include their index and available language metadata. Starting delays are retained, and each stream can be edited, processed, exported, saved, and reopened independently. The original video is stored once. Videos without audio can be imported and played with a silent audio timeline.

Video is displayed by WebKitGTK on Linux and WebView2 on Windows. Linux MP4/MOV and Matroska/WebM previews hide audio tracks in the metadata served to the video element, so WebKit decodes only the picture while the Rust engine plays audio. This preserves byte positions, HTTP ranges, the original source, and the saved project without transcoding or another video copy. Windows receives the original video bytes. Preview availability depends on installed codecs; an unavailable decoder produces a localized message. Frequent seeks keep the most recent requested position while an earlier decoder seek finishes. Playback waits for the requested video frame before resuming audio and video together. This wait is limited to five seconds so an unavailable preview does not prevent audio playback. Decoder compensation is measured and bounded. Playback that advances before the browser's play promise completes still synchronizes with audio. Preview holds its final frame when the video media ends during an active clip. Export produces audio; it preserves the original video in the project.

Only one audio track can be armed for microphone recording. Recording starts at the playhead while the other tracks play. Finishing a take stops playback. Original takes are stored without effects; processing is applied during playback and export.

Lock protects clip editing while allowing recording, mixer changes, and effects. Mute affects playback; it does not mute the recorded input. Split at a click or the playhead with 48 kHz sample accuracy. Drag the middle of the lower scrollbar to pan, or its edges to zoom. The scrollbar handle has a minimum width of 60 px.

Solo includes only tracks with Solo enabled in the playback mix. Other tracks are dimmed in the timeline and mixer; hover identifies the tracks with Solo enabled. Multiple tracks can be soloed together. Individual track exports remain independent of Mute/Solo, and their suggested filename uses the track name.

Hold the middle mouse button over a track and drag horizontally to pan the timeline. Add track creates an unarmed audio track. Voice clips can move to audio tracks; video clips stay on video tracks. Locked source or destination tracks prevent moving clips.

Drag through empty track space, or hold Shift and drag over clips, to select a time interval across several tracks. Ctrl+click toggles whole clips in a group. Ctrl+C copies selected sections with their source offsets, spacing, and relative track positions. Ctrl+V pastes at the playhead, starting on the selected track; enough compatible, unlocked destination tracks must exist. Copying retains references to the existing media and does not duplicate audio files. Drag a selected clip or the narrow top edge of the selection to move the group. Moving or deleting an interval preserves audio outside that interval and creates one Undo step. The internal clipboard belongs to the current project.

## Language and application preferences

Settings contains the interface language selector. The default follows the operating system language exposed by WebView2, with English as the fallback. Available languages: English (`en`), Russian (`ru`), French (`fr`), Polish (`pl`), Spanish (`es`), Portuguese (`pt`), German (`de`), Italian (`it`), Ukrainian (`uk`), Turkish (`tr`), Japanese (`ja`), Korean (`ko`), and Simplified Chinese (`zh-CN`). All use left-to-right layout.

Input and output selectors each include System Default. Linux routing uses the Pulse API with PulseAudio or PipeWire's Pulse server. The application checks device changes every two seconds as well as listening for browser device events. System Default follows system changes during playback and recording; an explicit choice remains pinned while that device is available. An unavailable selected device falls back to the system default.

Language changes apply immediately. Preferences are saved independently of projects in `%LOCALAPPDATA%\simpleVoiceover\settings.json` on Windows. Headless Linux uses `~/.local/share/simpleVoiceover`; `--config-dir` overrides the directory. Opening, saving, or moving a project does not replace the language preference.

Shared translation catalogs live in `src/locales`. English is complete and supplies runtime fallback for missing or empty entries. Rust errors and progress carry localization keys and parameters; the UI translates them. Native file dialogs use the same catalogs. Default project and track names are stored as keys, while custom names and source filenames remain user data. Legacy default Russian names are recognized through the Russian catalog. Update every catalog when adding a key. Tests check key parity, parameter parity, complete values, key references, and the absence of Cyrillic text outside localization files.

## Project storage and cache

A `.justspeak` v3 project is one portable indexed file, without compression. It includes source media, recordings, PCM cache, waveforms, edits, and track settings. Version 1.2 opens older ZIP64 v2 projects and migrates them when saving changes. The initial migration copies media once. Version 3 projects require v1.2 or a newer compatible application.

- Before the first save, the cache is in the chosen working directory, defaulting to `%TEMP%\simpleVoiceover`.
- After saving, changes create a `.simpleVoiceover-<UUID>` directory beside the project. It contains a change manifest and only new media or takes. Saved media is read directly from the project during editing.
- Ctrl+S appends new media, edits, and an index while reusing existing media regions. Saving edits does not copy audio or video. External cache and Undo/Redo history are cleared after a successful save.
- The index is read once when opening. Playback, HTTP Range, and export use 64-bit offsets without extraction.
- Two alternating pointers with CRC32 protect save commits. Data and the index are synced before updating a pointer. Incomplete appends are ignored; a damaged new revision leaves the previous intact revision accessible.
- Ordinary saves retain old manifests and removed media inside the file. Save as compacts only current data, including when choosing the same path, and atomically replaces the destination.
- An unfinished session is recovered at startup. Closing without saving discards its changes and removes its owned temporary cache.

## Export and effects

Export a track or the mix through a native dialog as WAV or MP3. WAV uses stereo 16-bit PCM at 48 kHz, with RF64 for large files. MP3 uses stereo 256 kbps at 48 kHz through shine-rs. Individual track export ignores mute/solo and preserves leading silence for alignment. Mix export respects mute/solo. Both apply edits, effects, volume, and pan.

The Effects tab offers Natural voice, Neutral, Warm voice, Clear voice, Podcast, and Noisy room presets. Selecting a preset applies its processing immediately. Editing its settings changes the dropdown to Custom Preset. Reset restores the selected preset. Save current settings as preset stores the current effects and bypass state under a custom name in the application settings, independently of the project. Saving an existing name updates that preset. Up to 64 custom presets are supported.

The adjustable Natural voice preset uses high-pass at 70 Hz, EQ at 250 Hz / −1.5 dB, presence at 3200 Hz / +1 dB, low-pass at 15 kHz, RMS compression at −24 dB / 2.3:1, attack 12 ms, release 160 ms, a soft knee corresponding to 2.5 linear, and makeup gain +2.9 dB. The RMS detector controls compression directly, without a second gain-smoothing stage. A soft expander attenuates quiet sections without removing time; attenuation is disabled by default. Whisper and spectral noise reduction are not used.

Voice presets enable loudness normalization, initially targeting −16 LUFS and a −1.5 dBTP true peak ceiling. Both values are adjustable. Before playback or export, Rust measures the processed track with EBU R128 gating and oversampled true peaks, then applies a constant gain during playback and export. If peak headroom prevents reaching the target, the gain remains lower; the app does not reshape dynamics to force the target. Gain is capped at +30 dB, and silence is left unchanged. Measurements use bounded histogram storage and are cached by source regions and processing settings; track names, faders, target changes, and shifting an entire track reuse the analysis. Changed processing is remeasured at the next playback start or export. No media is rewritten. Normalization precedes the manual volume and pan controls; changing faders or mixing tracks can change final peaks. Microphone monitoring uses RMS processing without loudness normalization because the future recording is not yet known. Legacy effects load with normalization disabled until enabled or a new preset is applied.

Every effect and mixer slider has a question-mark button. Hover or focus shows a localized explanation and a small schematic that follows the setting. Schematics illustrate parameter behavior; they are not measurements of the recording.

## Keyboard

| Keys                  | Action                           |
| --------------------- | -------------------------------- |
| Space                 | Play / pause                     |
| R                     | Start / finish recording         |
| V / X                 | Select / split tool              |
| Delete                | Remove selected clip             |
| Home                  | Go to start                      |
| Ctrl+S / Ctrl+Shift+S | Save / save as                   |
| Ctrl+Z / Ctrl+Shift+Z | Undo / redo                      |
| Ctrl+C / Ctrl+V       | Copy section / paste at playhead |
| Ctrl+ + / Ctrl+ −     | Horizontal timeline zoom         |
| Ctrl+wheel up / down  | Zoom in / out at the pointer     |

## Architecture and checks

React components live in `src/components`; timeline UI lives in `src/timeline`. Hooks separate transport, shortcuts, native file drop, and window lifecycle. `StudioClient` uses Tauri commands and events. Binary WebSocket carries PCM, microphone input, and delivery acknowledgements. AudioWorklet performs physical audio capture and output.

Rust separates command control, editing, storage, legacy ZIP64 reading, indexed archives, copying media readers, progress, native dialogs, decoding, DSP, mixing, sessions, HTTP/Range, localization, and Tauri lifecycle. PCM is written in blocks, the decoder reuses buffers, and 48 kHz input bypasses resampling. HTTP API does not expose project control.

```bash
npm test
npm run format:check
npm run build
cargo fmt --manifest-path backend/Cargo.toml --check
cargo clippy --manifest-path backend/Cargo.toml --tests -- -D warnings
```

Unit and integration tests cover sample-accurate splitting, exclusive recording arm, lock/mute, recording and final microphone data, Undo/Redo and history cleanup, lazy cache and recovery, append-only media saves, interrupted saves, asset removal and compaction, ZIP64 migration, portable Save as, seeks beyond 4 GiB, progress, failed-import cleanup, WAV/MP3 roundtrip, HTTP Range, origin restrictions, WebSocket, AudioWorklet I/O, and localization. A full 100 GB project has not been stress-tested.

A reproducible ignored benchmark measures import, first save, edit, and metadata-only save with ten minutes of stereo WAV:

```bash
cargo test --manifest-path backend/Cargo.toml large_project_import_and_metadata_save -- --ignored --nocapture
```

Media libraries: Symphonia (MPL-2.0), shine-rs (LGPL-2.0), zip (MIT), ebur128 (MIT). Exact versions are locked in `backend/Cargo.lock`; frontend dependencies are locked in `package-lock.json`.
