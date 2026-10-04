fn main() {
    #[cfg(feature = "desktop")]
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&["studio_command", "close_studio"]),
    ))
    .expect("Failed to prepare Tauri application");
}
