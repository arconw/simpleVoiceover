use crate::{
    commands, media, mixer,
    model::{Asset, MAX_UPLOAD, SAMPLE_RATE, id},
    storage::Store,
};
use anyhow::Result;
use serde_json::json;
use std::path::PathBuf;

#[test]
fn native_media_edit_save_playback_and_export_roundtrip() -> Result<()> {
    let temporary = tempfile::tempdir()?;
    let mut store = Store::open(
        temporary.path().join("config"),
        Some(temporary.path().join("work")),
    )?;
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/studio-check.mp4");
    commands::import_source(
        &mut store,
        fixture,
        "video.mp4".into(),
        "video".into(),
        None,
        0.,
    )?;
    assert!((store.project.duration() - 6.).abs() < 0.1);
    let track_id = store.project.tracks[0].id.clone();
    let clip_id = store.project.tracks[0].clips[0].id.clone();
    commands::execute(
        &mut store,
        &json!({"command":"clip_split","trackId":track_id,"clipId":clip_id,"position":2.}),
    )?;
    assert_eq!(store.project.tracks[0].clips.len(), 2);
    commands::execute(
        &mut store,
        &json!({"command":"track_patch","trackId":track_id,"patch":{"volume":-6.,"pan":-0.25}}),
    )?;
    let file = temporary.path().join("saved").join("session.justspeak");
    store.save(&file)?;
    assert!(!store.root().exists());
    let mut renderer = mixer::Mixer::new(&store)?;
    let (samples, _) = renderer.block(&store.project, 10000, 1024, None, None)?;
    assert!(samples.iter().flatten().any(|v| v.abs() > 0.01));
    drop(renderer);
    let wav = temporary.path().join("track.wav");
    let mp3 = temporary.path().join("track.mp3");
    mixer::render(&store, &wav, Some(&track_id), "wav")?;
    mixer::render(&store, &mp3, Some(&track_id), "mp3")?;
    for source in [wav, mp3] {
        let asset = Asset {
            id: id(),
            name: source.file_name().unwrap().to_string_lossy().into_owned(),
            kind: "audio".into(),
            size: std::fs::metadata(&source)?.len(),
            duration: 0.,
            frames: 0,
            sample_rate: SAMPLE_RATE,
            waveform: vec![],
            peak_frames: 256,
        };
        let decoded = media::decode(&source, &temporary.path().join("check.pcm"), asset)?;
        assert!((decoded.duration - store.project.duration()).abs() < 0.15);
        assert!(decoded.waveform.iter().any(|p| p[1] > 0.01));
    }
    assert!(!store.root().exists());
    commands::execute(
        &mut store,
        &json!({"command":"track_patch","trackId":track_id,"patch":{"volume":-3.}}),
    )?;
    assert!(store.root().starts_with(file.parent().unwrap()));
    assert!(!store.pcm(&store.project.assets[0].id).exists());
    let mut renderer = mixer::Mixer::new(&store)?;
    let (samples, _) = renderer.block(&store.project, 10000, 1024, None, None)?;
    assert!(samples.iter().flatten().any(|sample| sample.abs() > 0.01));
    drop(renderer);
    store.save(&file)?;
    assert!(!store.root().exists());
    let restarted = Store::open(temporary.path().join("config"), None)?;
    assert!(!restarted.config.dirty);
    assert_eq!(restarted.project.tracks[0].volume, -3.);
    assert_eq!(MAX_UPLOAD, 100_000_000_000);
    Ok(())
}

#[test]
fn invalid_edits_never_change_saved_project() -> Result<()> {
    let temporary = tempfile::tempdir()?;
    let mut store = Store::open(
        temporary.path().join("config"),
        Some(temporary.path().join("work")),
    )?;
    let track_id = store.project.tracks[1].id.clone();
    let project_file = temporary.path().join("project.justspeak");
    store.save(&project_file)?;
    assert!(
        commands::execute(
            &mut store,
            &json!({"command":"track_patch","trackId":track_id,"patch":{"volume":999.}})
        )
        .is_err()
    );
    assert!(!store.config.dirty);
    assert!(!store.root().exists());
    assert!(
        commands::execute(
            &mut store,
            &json!({"command":"track_patch","trackId":track_id,"patch":{"clips":[]}})
        )
        .is_err()
    );
    Ok(())
}

fn fixture_store() -> Result<(tempfile::TempDir, Store)> {
    let temporary = tempfile::tempdir()?;
    let mut store = Store::open(
        temporary.path().join("config"),
        Some(temporary.path().join("work")),
    )?;
    commands::import_source(
        &mut store,
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/studio-check.mp4"),
        "video.mp4".into(),
        "video".into(),
        None,
        0.,
    )?;
    Ok((temporary, store))
}

#[test]
fn removing_tracks_is_undoable_and_preserves_shared_media() -> Result<()> {
    let (_temporary, mut store) = fixture_store()?;
    let mut duplicate = store.project.tracks[0].clone();
    duplicate.id = id();
    duplicate.clips[0].id = id();
    let mut project = store.project.clone();
    project.tracks.push(duplicate.clone());
    store.replace(project)?;
    let original = serde_json::to_value(&store.project)?;
    let removed = store.project.tracks[0].id.clone();
    let asset = store.project.assets[0].id.clone();
    commands::execute(
        &mut store,
        &json!({"command":"track_remove","trackId":removed}),
    )?;
    assert_eq!(store.project.tracks.len(), 3);
    assert_eq!(store.project.assets.len(), 1);
    assert!(store.source(&asset).exists());
    let mut renderer = mixer::Mixer::new(&store)?;
    let (samples, _) = renderer.block(&store.project, 10000, 1024, Some(&duplicate.id), None)?;
    assert!(samples.iter().flatten().any(|sample| sample.abs() > 0.01));
    drop(renderer);
    commands::execute(&mut store, &json!({"command":"undo"}))?;
    assert_eq!(serde_json::to_value(&store.project)?, original);
    commands::execute(&mut store, &json!({"command":"redo"}))?;
    assert!(!store.project.tracks.iter().any(|track| track.id == removed));
    commands::execute(&mut store, &json!({"command":"undo"}))?;
    assert_eq!(serde_json::to_value(&store.project)?, original);
    Ok(())
}

#[test]
fn invalid_and_locked_track_removal_preserve_saved_state_and_history() -> Result<()> {
    let (temporary, mut store) = fixture_store()?;
    let track = store.project.tracks[0].id.clone();
    commands::execute(
        &mut store,
        &json!({"command":"track_patch","trackId":track,"patch":{"locked":true}}),
    )?;
    store.save(&temporary.path().join("locked.justspeak"))?;
    let original = serde_json::to_value(&store.project)?;
    for request in [
        json!({"command":"track_remove","trackId":track}),
        json!({"command":"track_remove","trackId":"missing"}),
        json!({"command":"track_remove"}),
    ] {
        assert!(commands::execute(&mut store, &request).is_err());
        assert_eq!(serde_json::to_value(&store.project)?, original);
        assert!(!store.config.dirty);
        assert!(store.history.is_empty() && store.redo.is_empty());
        assert!(!store.root().exists());
    }
    Ok(())
}

#[test]
fn removing_the_last_track_can_be_saved_reopened_and_followed_by_adding_a_track() -> Result<()> {
    let (temporary, mut store) = fixture_store()?;
    let ids: Vec<_> = store
        .project
        .tracks
        .iter()
        .map(|track| track.id.clone())
        .collect();
    for track in ids {
        commands::execute(
            &mut store,
            &json!({"command":"track_remove","trackId":track}),
        )?;
    }
    assert!(store.project.tracks.is_empty());
    let project_id = store.project.id.clone();
    let asset_id = store.project.assets[0].id.clone();
    let file = temporary.path().join("empty.justspeak");
    store.save(&file)?;
    drop(store);
    let mut reopened = Store::open(temporary.path().join("config"), None)?;
    assert_eq!(reopened.project.id, project_id);
    assert!(reopened.project.tracks.is_empty());
    assert_eq!(reopened.project.assets[0].id, asset_id);
    commands::execute(&mut reopened, &json!({"command":"track_add"}))?;
    assert_eq!(reopened.project.tracks.len(), 1);
    assert_eq!(reopened.project.tracks[0].kind, "audio");
    assert!(reopened.config.dirty);
    Ok(())
}

#[test]
fn track_removal_cannot_interrupt_a_recording() -> Result<()> {
    let (_temporary, mut store) = fixture_store()?;
    let mut session = crate::session::AudioSession::new();
    crate::controller::execute(
        &mut store,
        &mut session,
        &json!({"command":"record_begin","position":0.}),
    )?;
    let original = serde_json::to_value(&store.project)?;
    let track = store.project.tracks[1].id.clone();
    assert!(
        crate::controller::execute(
            &mut store,
            &mut session,
            &json!({"command":"track_remove","trackId":track})
        )
        .is_err()
    );
    assert!(session.recording.is_some());
    assert_eq!(serde_json::to_value(&store.project)?, original);
    Ok(())
}

#[test]
fn solo_includes_all_selected_tracks_and_mute_still_silences_them() -> Result<()> {
    let (_temporary, mut store) = fixture_store()?;
    let audible = store.project.tracks[0].id.clone();
    let empty = store.project.tracks[1].id.clone();
    commands::execute(
        &mut store,
        &json!({"command":"track_patch","trackId":empty,"patch":{"solo":true}}),
    )?;
    let mut renderer = mixer::Mixer::new(&store)?;
    let (silenced, _) = renderer.block(&store.project, 10000, 1024, None, None)?;
    assert!(silenced.iter().flatten().all(|sample| *sample == 0.));
    commands::execute(
        &mut store,
        &json!({"command":"track_patch","trackId":audible,"patch":{"solo":true}}),
    )?;
    let (both, _) = renderer.block(&store.project, 11024, 1024, None, None)?;
    assert!(both.iter().flatten().any(|sample| sample.abs() > 0.01));
    commands::execute(
        &mut store,
        &json!({"command":"track_patch","trackId":audible,"patch":{"mute":true}}),
    )?;
    let (muted, _) = renderer.block(&store.project, 12048, 1024, None, None)?;
    assert!(muted.iter().flatten().all(|sample| *sample == 0.));
    Ok(())
}

#[test]
fn arm_is_exclusive_and_lock_only_protects_clip_editing() -> Result<()> {
    let (_temporary, mut store) = fixture_store()?;
    let voice = store.project.tracks[1].id.clone();
    let other = store.project.tracks[2].id.clone();
    commands::execute(
        &mut store,
        &json!({"command":"track_patch","trackId":voice,"patch":{"locked":true}}),
    )?;
    assert!(store.project.tracks[1].armed);
    commands::execute(
        &mut store,
        &json!({"command":"track_patch","trackId":other,"patch":{"armed":true}}),
    )?;
    assert!(!store.project.tracks[1].armed);
    assert!(store.project.tracks[2].armed);
    commands::execute(
        &mut store,
        &json!({"command":"track_patch","trackId":voice,"patch":{"armed":true,"mute":true,"volume":-6.}}),
    )?;
    assert_eq!(
        store
            .project
            .tracks
            .iter()
            .filter(|track| track.armed)
            .count(),
        1
    );
    let mut session = crate::session::AudioSession::new();
    crate::controller::execute(
        &mut store,
        &mut session,
        &json!({"command":"record_begin","position":2.125}),
    )?;
    assert!(session.playing);
    let (packet, _) = session.pull(&store)?;
    assert_eq!(f64::from_le_bytes(packet[8..16].try_into()?), 2.125);
    assert!(
        packet[16..]
            .chunks_exact(4)
            .any(|sample| f32::from_le_bytes(sample.try_into().unwrap()).abs() > 0.01)
    );
    let mut microphone = vec![1];
    for _ in 0..1024 {
        microphone.extend(0.375f32.to_le_bytes());
    }
    session.binary(&microphone)?;
    assert!(
        crate::controller::execute(
            &mut store,
            &mut session,
            &json!({"command":"save","path":"unused"})
        )
        .is_err()
    );
    crate::controller::execute(&mut store, &mut session, &json!({"command":"record_end"}))?;
    assert!(!session.playing && session.recording.is_none());
    let clip = &store.project.tracks[1].clips[0];
    assert_eq!(clip.start, 2.125);
    assert_eq!(clip.duration, 1024. / SAMPLE_RATE as f64);
    let pcm = std::fs::read(store.pcm(&clip.asset_id))?;
    assert!(
        pcm.chunks_exact(4)
            .all(|sample| f32::from_le_bytes(sample.try_into().unwrap()) == 0.375)
    );
    let clip_id = clip.id.clone();
    assert!(
        commands::execute(
            &mut store,
            &json!({"command":"clip_remove","trackId":voice,"clipId":clip_id})
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn mute_and_solo_only_affect_mix_and_not_individual_export() -> Result<()> {
    let (temporary, mut store) = fixture_store()?;
    let track_id = store.project.tracks[0].id.clone();
    commands::execute(
        &mut store,
        &json!({"command":"track_patch","trackId":track_id,"patch":{"mute":true}}),
    )?;
    let mut mixer = mixer::Mixer::new(&store)?;
    let (mixed, _) = mixer.block(&store.project, 10000, 1024, None, None)?;
    assert!(mixed.iter().flatten().all(|sample| *sample == 0.));
    let (solo_export, _) = mixer.block(&store.project, 10000, 1024, Some(&track_id), None)?;
    assert!(
        solo_export
            .iter()
            .flatten()
            .any(|sample| sample.abs() > 0.01)
    );
    let destination = temporary.path().join("muted-track.wav");
    commands::execute(
        &mut store,
        &json!({"command":"export","trackId":track_id,"format":"wav","path":destination}),
    )?;
    assert!(std::fs::metadata(destination)?.len() > 10000);
    Ok(())
}

#[test]
fn cuts_are_sample_exact_and_save_clears_both_histories() -> Result<()> {
    let (temporary, mut store) = fixture_store()?;
    let track = store.project.tracks[0].id.clone();
    let clip = store.project.tracks[0].clips[0].id.clone();
    let cut = 0.0123456;
    commands::execute(
        &mut store,
        &json!({"command":"clip_split","trackId":track,"clipId":clip,"position":cut}),
    )?;
    let expected = (cut * SAMPLE_RATE as f64).round() / SAMPLE_RATE as f64;
    assert_eq!(store.project.tracks[0].clips[1].start, expected);
    assert_eq!(store.project.tracks[0].clips[1].offset, expected);
    commands::execute(
        &mut store,
        &json!({"command":"clip_edit","trackId":track,"clipId":clip,"mode":"left","delta":0.}),
    )?;
    assert_eq!(store.project.tracks[0].clips[0].duration, expected);
    commands::execute(&mut store, &json!({"command":"undo"}))?;
    commands::execute(&mut store, &json!({"command":"undo"}))?;
    assert_eq!(store.project.tracks[0].clips.len(), 1);
    commands::execute(&mut store, &json!({"command":"redo"}))?;
    assert_eq!(store.project.tracks[0].clips.len(), 2);
    commands::execute(&mut store, &json!({"command":"undo"}))?;
    assert!(!store.history.is_empty() && !store.redo.is_empty());
    store.save(&temporary.path().join("session.justspeak"))?;
    assert!(store.history.is_empty() && store.redo.is_empty());
    commands::execute(&mut store, &json!({"command":"undo"}))?;
    commands::execute(&mut store, &json!({"command":"redo"}))?;
    assert_eq!(store.project.tracks[0].clips.len(), 1);
    assert!(!store.root().exists());
    Ok(())
}

#[test]
fn close_guard_preserves_recording_and_discards_only_unsaved_changes() -> Result<()> {
    let (temporary, mut store) = fixture_store()?;
    let file = temporary.path().join("session.justspeak");
    store.save(&file)?;
    let track = store.project.tracks[0].id.clone();
    commands::execute(
        &mut store,
        &json!({"command":"track_patch","trackId":track,"patch":{"volume":-6.}}),
    )?;
    let cache = store.root();
    let mut session = crate::session::AudioSession::new();
    assert!(crate::lifecycle::prepare_close(&mut store, &session, false).is_err());
    assert!(cache.exists());
    session.begin_record(&mut store, 1.)?;
    assert!(crate::lifecycle::prepare_close(&mut store, &session, true).is_err());
    session.finish_record(&mut store)?;
    crate::lifecycle::prepare_close(&mut store, &session, true)?;
    assert!(!cache.exists());
    assert!(!store.config.dirty);
    assert_eq!(store.project.tracks[0].volume, 0.);
    assert!(file.exists());
    Ok(())
}
