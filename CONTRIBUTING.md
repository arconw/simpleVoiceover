# Contributing

Thanks for helping with simpleVoiceover. Bug reports, small fixes, translations and platform improvements are all welcome.

## Before you start

- Search the [issues](https://github.com/arconw/simpleVoiceover/issues) first. For a larger change, open an issue to agree on the approach before writing code.
- Report security problems privately, as described in [SECURITY.md](SECURITY.md).
- Everyone taking part follows the [Code of Conduct](CODE_OF_CONDUCT.md).

## Set up

You need Node.js 24 and Rust 1.89 or newer. On Debian and Ubuntu the desktop build also needs the system libraries listed in the [README](README.md#build-from-source).

```bash
npm ci
npm run build
npm test
```

## Checks

Run these before opening a pull request. CI runs the same checks on Linux, Windows and macOS.

```bash
npm run format:check
npm run lint
npm run build
npm test
cargo fmt --manifest-path backend/Cargo.toml --check
```

`npm run lint` runs ESLint and Clippy with warnings treated as errors. `npm test` runs the TypeScript unit tests and the Rust tests. The Linux compatibility test bench in `linux-testbench` is optional and not part of CI.

## Conventions

- React and TypeScript components live in `src/components` and timeline behavior in `src/timeline`. Rust in `backend/src` owns decoding, DSP, project storage and editing.
- All user-facing text lives in `src/locales`. English is the complete fallback. Update every supported language and keep interpolation parameters unchanged.
- Documentation is written in English.
- Do not add code comments. Prefer clear names and focused functions.
- Add meaningful tests for storage, protocol and localization changes.
- Record user-visible changes under `Unreleased` in [CHANGELOG.md](CHANGELOG.md).

## Pull requests

- Keep each pull request to one concern and describe what changed and why.
- Make sure CI is green. Mention the platform you tested on.
- Do not commit build output. Anything in `build/` stays out of Git.

## Releases

Only maintainers change the application version or publish releases. See [docs/RELEASING.md](docs/RELEASING.md).
