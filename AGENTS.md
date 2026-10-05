# Project instructions

- React and TypeScript components live in `src/components`; timeline behavior lives in `src/timeline`. Hooks coordinate transport and desktop lifecycle through `StudioClient` and Tauri IPC. Rust in `backend/src` owns decoding, DSP, project storage, and editing; WebSocket carries binary audio only.
- Run `npm test` for TypeScript unit tests and Rust integration tests. Validate changes with `npm run build`, `npm run format:check`, and `cargo fmt --manifest-path backend/Cargo.toml --check`. Add meaningful tests for storage, protocol, and localization changes.
- Keep all user-facing text in `src/locales`. English is the complete fallback catalog. Update every supported language and preserve interpolation parameters whenever translations change. Keep documentation in English.
- Do not add code comments. Use clear names and focused functions.
