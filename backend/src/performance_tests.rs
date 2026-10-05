use crate::{commands, media::wav_header, model::SAMPLE_RATE, storage::Store};
use anyhow::Result;
use serde_json::json;
use std::{
    fs::{self, File},
    io::Write,
    time::Instant,
};

#[test]
#[ignore = "Writes a 10-minute WAV and benchmarks large-project import and saves"]
fn large_project_import_and_metadata_save() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let source = temp.path().join("voice.wav");
    let frames = SAMPLE_RATE as u64 * 600;
    let mut file = File::create(&source)?;
    file.write_all(&wav_header(frames))?;
    let block: Vec<_> = [0, 32, 0, 32]
        .into_iter()
        .cycle()
        .take(1024 * 1024)
        .collect();
    let mut remaining = frames * 4;
    while remaining > 0 {
        let count = remaining.min(block.len() as u64) as usize;
        file.write_all(&block[..count])?;
        remaining -= count as u64;
    }
    drop(file);
    let mut store = Store::open(temp.path().join("config"), Some(temp.path().join("work")))?;
    let start = Instant::now();
    commands::import_source(
        &mut store,
        source,
        "voice.wav".into(),
        "audio".into(),
        None,
        0.,
    )?;
    let import_ms = start.elapsed().as_secs_f64() * 1000.;
    let destination = temp.path().join("project.justspeak");
    let start = Instant::now();
    store.save(&destination)?;
    let first_save_ms = start.elapsed().as_secs_f64() * 1000.;
    let initial_size = fs::metadata(&destination)?.len();
    let track = store.project.tracks[2].id.clone();
    let start = Instant::now();
    commands::execute(
        &mut store,
        &json!({"command":"track_patch","trackId":track,"patch":{"volume":-3.}}),
    )?;
    let edit_ms = start.elapsed().as_secs_f64() * 1000.;
    let cache_bytes: u64 = ["media", "cache"]
        .into_iter()
        .map(|directory| -> Result<u64> {
            fs::read_dir(store.root().join(directory))?
                .map(|entry| Ok(entry?.metadata()?.len()))
                .sum()
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .sum();
    let start = Instant::now();
    store.save(&destination)?;
    let metadata_save_ms = start.elapsed().as_secs_f64() * 1000.;
    let final_size = fs::metadata(&destination)?.len();
    println!(
        "import_ms={import_ms:.1} first_save_ms={first_save_ms:.1} edit_ms={edit_ms:.1} metadata_save_ms={metadata_save_ms:.1} project_bytes={initial_size} cache_bytes={cache_bytes} growth_bytes={}",
        final_size.saturating_sub(initial_size)
    );
    assert_eq!(store.project.assets[0].frames, frames);
    assert!(!store.root().exists());
    Ok(())
}
