use anyhow::{Context, Result};
use std::sync::Mutex;
use tempfile::TempDir;

const PLUGIN: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/libgstsvoiceaudio.so"));
static DIRECTORY: Mutex<Option<TempDir>> = Mutex::new(None);

pub fn initialize_environment() -> Result<()> {
    let directory = tempfile::Builder::new()
        .prefix("simplevoiceover-audio-")
        .tempdir()
        .context("Failed to prepare Linux audio output")?;
    std::fs::write(directory.path().join("libgstsvoiceaudio.so"), PLUGIN)
        .context("Failed to write Linux audio output plugin")?;
    let existing =
        std::env::var_os("GST_PLUGIN_PATH_1_0").or_else(|| std::env::var_os("GST_PLUGIN_PATH"));
    let paths = std::iter::once(directory.path().to_path_buf()).chain(
        existing
            .as_deref()
            .map(std::env::split_paths)
            .into_iter()
            .flatten()
            .filter(|path| !path.as_os_str().is_empty()),
    );
    let plugins = std::env::join_paths(paths).context("Invalid GStreamer plugin path")?;
    let registry = directory.path().join("registry.bin");
    unsafe {
        std::env::set_var("GST_PLUGIN_PATH_1_0", plugins);
        std::env::set_var("GST_REGISTRY_1_0", registry);
    }
    *DIRECTORY
        .lock()
        .map_err(|_| anyhow::anyhow!("Failed to retain Linux audio output plugin"))? =
        Some(directory);
    Ok(())
}

pub fn cleanup() {
    if let Ok(mut directory) = DIRECTORY.lock() {
        directory.take();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn webaudio_preserves_clock_sync_and_has_a_separate_system_mixer_identity() {
        let registry = tempfile::tempdir().unwrap();
        let output = std::process::Command::new(concat!(env!("OUT_DIR"), "/audio-output-tests"))
            .arg(concat!(env!("OUT_DIR"), "/libgstsvoiceaudio.so"))
            .env("GST_REGISTRY_1_0", registry.path().join("registry.bin"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
