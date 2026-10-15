# Changelog

All notable changes to simpleVoiceover are recorded here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [1.3.1] - 2026-10-15

### Added

- macOS support, contributed by Emrah Diril: an ad-hoc signed `.app` and `.dmg` built with `bin/build-macos`, microphone access limited to the studio page, and settings stored in `~/.simpleVoiceover`. There are no prebuilt macOS downloads yet.
- Continuous integration for every push to `master` and every pull request: formatting, ESLint, Clippy, the interface build, unit tests and engine tests on Linux, Windows and macOS.
- A release workflow that builds all four packages from a `v*` tag and prepares a draft GitHub Release with `checksums.sha256`.
- Optional Windows code signing through SignPath, enabled when the repository defines `SIGNPATH_ORGANIZATION_ID`.

### Changed

- The initial window fits inside the screen's work area instead of opening at a fixed 1440×900.
- Video preview playback recovers from a refused `play()` after the first click or key press and resumes at the current transport position.
- The minimum supported Rust version is declared as 1.89.

### Fixed

- Saving a project into its own file a second time no longer fails on Windows with a locked-file error.

## [1.3.0] - 2026-10-10

### Added

- Input and output device selection in Settings, including System Default, with live handling of device removal and reconnection on Linux.
- Import of H.264 MP4 and Matroska files with every audio stream on its own track, keeping stream order, language and starting delay.
- Magnetic clip snapping, confirmed track deletion with Undo, immediate effect presets, named custom presets and Solo indicators.
- AppImage, DEB and RPM packages built on the Ubuntu 22.04 / glibc 2.35 baseline, alongside the Windows EXE.
- A deployable Linux compatibility test bench in `linux-testbench`.

### Changed

- Video seeking and audio/video synchronization are more reliable; obsolete seek requests are cancelled.
- Exported files are named after the track or project.

### Fixed

- Double-clicking the preview no longer crashes fullscreen mode.

## [1.2.1] - 2026-10-08

### Added

- The first published release: a Windows executable and a Debian/Ubuntu package.

[Unreleased]: https://github.com/arconw/simpleVoiceover/compare/v1.3.1...HEAD
[1.3.1]: https://github.com/arconw/simpleVoiceover/compare/v1.3.0...v1.3.1
[1.3.0]: https://github.com/arconw/simpleVoiceover/compare/v1.2.1...v1.3.0
[1.2.1]: https://github.com/arconw/simpleVoiceover/releases/tag/v1.2.1
