fn main() {
    #[cfg(feature = "desktop")]
    {
        if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
            build_audio_output();
        }
        tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
            tauri_build::AppManifest::new().commands(&["studio_command", "close_studio"]),
        ))
        .expect("Failed to prepare Tauri application");
    }
}

#[cfg(feature = "desktop")]
fn build_audio_output() {
    let library = pkg_config::Config::new()
        .cargo_metadata(false)
        .probe("gstreamer-1.0")
        .expect("GStreamer development files are required for Linux audio output");
    compile_audio_native(
        "native/audio_output.c",
        "libgstsvoiceaudio.so",
        true,
        &library,
    );
    compile_audio_native(
        "native/audio_output_tests.c",
        "audio-output-tests",
        false,
        &library,
    );
}

#[cfg(feature = "desktop")]
fn compile_audio_native(source: &str, output: &str, shared: bool, library: &pkg_config::Library) {
    println!("cargo:rerun-if-changed={source}");
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join(output);
    let mut command = cc::Build::new()
        .cargo_metadata(false)
        .get_compiler()
        .to_command();
    command.args(["-O2", "-Wall", "-Wextra", "-Werror"]);
    if shared {
        command.args(["-shared", "-fPIC"]);
    }
    command.arg(source).arg("-o").arg(output);
    for include in &library.include_paths {
        command.arg("-I").arg(include);
    }
    for path in &library.link_paths {
        command.arg("-L").arg(path);
    }
    for library in &library.libs {
        command.arg(format!("-l{library}"));
    }
    for arguments in &library.ld_args {
        command.arg(format!("-Wl,{}", arguments.join(",")));
    }
    assert!(
        command
            .status()
            .expect("Failed to compile audio output")
            .success(),
        "Failed to compile audio output"
    );
}
