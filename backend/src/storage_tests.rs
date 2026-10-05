use crate::{
    archive::Archive, commands, media::wav_header, mixer::Mixer, progress::Progress, storage::Store,
};
use anyhow::Result;
use serde_json::json;
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
    sync::{Arc, Mutex},
};

fn new_store(path: &Path) -> Result<Store> {
    Store::open(path.join("config"), Some(path.join("work")))
}

fn import(store: &mut Store, path: &Path) -> Result<String> {
    let mut file = File::create(path)?;
    file.write_all(&wav_header(48000))?;
    for _ in 0..48000 {
        file.write_all(&[0, 32, 0, 32])?;
    }
    drop(file);
    commands::import_source(
        store,
        path.to_path_buf(),
        "voice.wav".into(),
        "audio".into(),
        None,
        0.,
    )?;
    Ok(store.project.assets.last().unwrap().id.clone())
}

fn bytes(store: &Store, asset: &str, pcm: bool) -> Result<Vec<u8>> {
    let (path, offset, length) = store.location(asset, pcm)?;
    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(offset))?;
    let mut bytes = Vec::new();
    file.take(length).read_to_end(&mut bytes)?;
    Ok(bytes)
}

#[test]
fn lazy_overlay_recovers_and_appends_only_new_media() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let mut store = new_store(temp.path())?;
    let asset = import(&mut store, &temp.path().join("one.wav"))?;
    let original = bytes(&store, &asset, false)?;
    let file = temp.path().join("project.justspeak");
    store.save(&file)?;
    let location = store.location(&asset, true)?;
    let initial_size = fs::metadata(&file)?.len();
    let track = store.project.tracks[2].id.clone();
    commands::execute(
        &mut store,
        &json!({"command":"track_patch","trackId":track,"patch":{"volume":-3.}}),
    )?;
    assert_eq!(fs::read_dir(store.root().join("media"))?.count(), 0);
    assert_eq!(fs::read_dir(store.root().join("cache"))?.count(), 0);
    assert_eq!(store.location(&asset, true)?, location);
    store.save(&file)?;
    assert!(fs::metadata(&file)?.len() - initial_size < 32 * 1024);
    assert_eq!(store.location(&asset, true)?, location);
    let second = import(&mut store, &temp.path().join("two.wav"))?;
    assert_eq!(fs::read_dir(store.root().join("media"))?.count(), 1);
    assert!(!store.source(&asset).exists());
    assert_eq!(bytes(&store, &asset, false)?, original);
    let mut reopened = Store::open(temp.path().join("config"), None)?;
    assert!(reopened.config.dirty);
    assert!(reopened.source(&second).exists());
    assert_eq!(reopened.location(&asset, true)?, location);
    let mut mixer = Mixer::new(&reopened)?;
    assert!(
        mixer
            .block(&reopened.project, 0, 1024, None, None)?
            .0
            .iter()
            .flatten()
            .any(|sample| *sample > 0.)
    );
    drop(mixer);
    let before = fs::metadata(&file)?.len();
    let new_payload = reopened.project.assets[1].size + reopened.project.assets[1].frames * 8;
    reopened.save(&file)?;
    let growth = fs::metadata(&file)?.len() - before;
    assert!(growth >= new_payload && growth < new_payload + 64 * 1024);
    assert_eq!(reopened.location(&asset, true)?, location);
    assert!(!reopened.root().exists());
    assert_eq!(Archive::open(&file)?.0.assets.len(), 2);
    Ok(())
}

#[test]
fn interrupted_append_and_torn_commit_preserve_previous_revision() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let mut store = new_store(temp.path())?;
    let asset = import(&mut store, &temp.path().join("voice.wav"))?;
    let file = temp.path().join("project.justspeak");
    store.save(&file)?;
    let previous = fs::read(&file)?;
    OpenOptions::new()
        .append(true)
        .open(&file)?
        .write_all(&vec![42; 1024 * 1024])?;
    assert_eq!(Archive::open(&file)?.0.assets[0].id, asset);
    store.begin_changes()?;
    store.project.name = "second".into();
    store.save(&file)?;
    assert!(fs::metadata(&file)?.len() < previous.len() as u64 + 32 * 1024);
    let revision = store
        .archive
        .as_ref()
        .unwrap()
        .revision
        .as_ref()
        .unwrap()
        .clone();
    let committed = fs::read(&file)?;
    let mut damaged = OpenOptions::new().write(true).open(&file)?;
    damaged.seek(SeekFrom::Start(48))?;
    damaged.write_all(&[0; 17])?;
    drop(damaged);
    assert_eq!(Archive::open(&file)?.0.name, "project.defaultName");
    fs::write(&file, &committed)?;
    let mut damaged = OpenOptions::new().write(true).open(&file)?;
    damaged.seek(SeekFrom::Start(revision.offset + 10))?;
    damaged.write_all(b"corrupt")?;
    drop(damaged);
    assert_eq!(Archive::open(&file)?.0.name, "project.defaultName");
    Ok(())
}

#[test]
fn failed_save_keeps_committed_media_and_pending_changes() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let mut store = new_store(temp.path())?;
    let asset = import(&mut store, &temp.path().join("one.wav"))?;
    let file = temp.path().join("project.justspeak");
    store.save(&file)?;
    let second = import(&mut store, &temp.path().join("two.wav"))?;
    fs::remove_file(store.pcm(&second))?;
    assert!(store.save(&file).is_err());
    assert!(store.config.dirty && store.root().exists());
    let saved = Archive::open(&file)?.0;
    assert_eq!(saved.assets.len(), 1);
    assert_eq!(saved.assets[0].id, asset);
    Ok(())
}

#[test]
fn removing_media_updates_all_tracks_supports_undo_and_compacts() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let mut store = new_store(temp.path())?;
    let asset = import(&mut store, &temp.path().join("voice.wav"))?;
    let track = store.project.tracks[1].id.clone();
    commands::execute(
        &mut store,
        &json!({"command":"clip_place","trackId":track,"assetId":asset,"position":2.5}),
    )?;
    assert_eq!(store.project.tracks[1].clips[0].start, 2.5);
    let file = temp.path().join("project.justspeak");
    store.save(&file)?;
    commands::execute(
        &mut store,
        &json!({"command":"track_patch","trackId":track,"patch":{"locked":true}}),
    )?;
    assert!(
        commands::execute(
            &mut store,
            &json!({"command":"asset_remove","assetId":asset})
        )
        .is_err()
    );
    commands::execute(
        &mut store,
        &json!({"command":"track_patch","trackId":track,"patch":{"locked":false}}),
    )?;
    commands::execute(
        &mut store,
        &json!({"command":"asset_remove","assetId":asset}),
    )?;
    assert!(store.project.assets.is_empty());
    assert!(
        store
            .project
            .tracks
            .iter()
            .all(|track| track.clips.is_empty())
    );
    commands::execute(&mut store, &json!({"command":"undo"}))?;
    assert_eq!(store.project.assets.len(), 1);
    assert_eq!(
        store
            .project
            .tracks
            .iter()
            .map(|track| track.clips.len())
            .sum::<usize>(),
        2
    );
    assert!(!bytes(&store, &asset, true)?.is_empty());
    commands::execute(&mut store, &json!({"command":"redo"}))?;
    store.save(&file)?;
    assert!(Archive::open(&file)?.0.assets.is_empty());
    let previous_size = fs::metadata(&file)?.len();
    store.save_with_options(&file, true)?;
    assert!(fs::metadata(&file)?.len() < previous_size / 2);
    assert!(store.archive.as_ref().unwrap().entries.is_empty());
    Ok(())
}

#[test]
fn legacy_zip_migrates_and_save_as_stays_portable() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let mut original = new_store(temp.path())?;
    let asset = import(&mut original, &temp.path().join("voice.wav"))?;
    let pcm = bytes(&original, &asset, true)?;
    let file = temp.path().join("legacy.justspeak");
    let mut zip = zip::ZipWriter::new(File::create(&file)?);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .large_file(true);
    original.project.version = 2;
    zip.start_file("project.json", options)?;
    serde_json::to_writer(&mut zip, &original.project)?;
    for (name, _) in crate::archive::asset_entries(&original.project) {
        zip.start_file(&name, options)?;
        zip.write_all(&fs::read(original.root().join(name))?)?;
    }
    zip.finish()?;
    original.load(&file)?;
    original.begin_changes()?;
    assert!(!original.pcm(&asset).exists());
    assert_eq!(bytes(&original, &asset, true)?, pcm);
    original.save(&file)?;
    assert_eq!(Archive::open(&file)?.0.version, 3);
    let destination = temp.path().join("portable.justspeak");
    original.save_with_options(&destination, true)?;
    fs::remove_file(&file)?;
    let mut portable = new_store(&temp.path().join("other"))?;
    portable.load(&destination)?;
    assert!(!portable.root().exists());
    assert_eq!(bytes(&portable, &asset, true)?, pcm);
    Ok(())
}

#[test]
fn import_failure_cleans_partial_files_and_progress_finishes() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let mut store = new_store(temp.path())?;
    let events = Arc::new(Mutex::new(Vec::new()));
    let received = events.clone();
    store.progress = Progress::new(move |event| received.lock().unwrap().push(event));
    let asset = import(&mut store, &temp.path().join("voice.wav"))?;
    let received = events.lock().unwrap();
    assert!(received.len() >= 2);
    assert_eq!(received.last().unwrap().percent, 100.);
    assert!(
        received
            .windows(2)
            .all(|pair| pair[0].percent <= pair[1].percent)
    );
    drop(received);
    let bad = temp.path().join("bad.mp4");
    fs::write(&bad, b"invalid media")?;
    assert!(
        commands::import_source(&mut store, bad, "bad.mp4".into(), "video".into(), None, 0.)
            .is_err()
    );
    assert_eq!(store.project.assets[0].id, asset);
    assert_eq!(fs::read_dir(store.root().join("media"))?.count(), 1);
    assert_eq!(fs::read_dir(store.root().join("cache"))?.count(), 1);
    Ok(())
}
