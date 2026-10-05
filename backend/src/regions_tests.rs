use crate::{
    commands,
    model::{Asset, Clip, SAMPLE_RATE, id},
    storage::Store,
};
use anyhow::Result;
use serde_json::{Value, json};

fn setup() -> Result<(tempfile::TempDir, Store)> {
    let directory = tempfile::tempdir()?;
    let mut store = Store::open(
        directory.path().join("config"),
        Some(directory.path().join("work")),
    )?;
    let asset = Asset {
        id: id(),
        name: "voice.wav".into(),
        kind: "audio".into(),
        size: 0,
        duration: 10.,
        frames: 10 * SAMPLE_RATE as u64,
        sample_rate: SAMPLE_RATE,
        waveform: vec![],
        peak_frames: 256,
    };
    store.project.tracks[1].clips.push(Clip {
        id: id(),
        asset_id: asset.id.clone(),
        name: asset.name.clone(),
        start: 1.,
        offset: 1.,
        duration: 8.,
    });
    store.project.tracks[2].clips.push(Clip {
        id: id(),
        asset_id: asset.id.clone(),
        name: asset.name.clone(),
        start: 2.,
        offset: 0.,
        duration: 5.,
    });
    store.project.assets.push(asset);
    store.project.validate()?;
    Ok((directory, store))
}

fn regions(store: &Store, from: f64, to: f64) -> Value {
    json!(
        store.project.tracks[1..]
            .iter()
            .map(|track| json!({"trackId":track.id,"clipId":track.clips[0].id,"from":from,"to":to}))
            .collect::<Vec<_>>()
    )
}

#[test]
fn moving_a_section_splits_all_tracks_and_is_one_undo_step() -> Result<()> {
    let (_directory, mut store) = setup()?;
    let original = serde_json::to_value(&store.project)?;
    let selected = regions(&store, 3., 5.);
    let result = commands::execute(
        &mut store,
        &json!({"command":"regions_edit","regions":selected,"delta":7.,"trackOffset":0}),
    )?;
    assert_eq!(store.history.len(), 1);
    assert_eq!(result["regions"].as_array().unwrap().len(), 2);
    for track in &store.project.tracks[1..] {
        assert_eq!(track.clips.len(), 3);
        let moved = track.clips.iter().find(|clip| clip.start == 10.).unwrap();
        assert_eq!(moved.duration, 2.);
        assert_eq!(moved.offset, if track.kind == "voice" { 3. } else { 1. });
        assert!(track.clips.iter().any(|clip| clip.start == 5.));
    }
    commands::execute(&mut store, &json!({"command":"undo"}))?;
    assert_eq!(serde_json::to_value(&store.project)?, original);
    commands::execute(&mut store, &json!({"command":"redo"}))?;
    assert_eq!(store.project.tracks[1].clips.len(), 3);
    Ok(())
}

#[test]
fn voice_clips_move_to_audio_tracks_and_add_only_creates_audio() -> Result<()> {
    let (_directory, mut store) = setup()?;
    let clip_id = store.project.tracks[1].clips[0].id.clone();
    let track_id = store.project.tracks[1].id.clone();
    commands::execute(
        &mut store,
        &json!({"command":"regions_edit","regions":[{"trackId":track_id,"clipId":clip_id,"from":1.,"to":9.}],"delta":2.,"trackOffset":1}),
    )?;
    assert!(store.project.tracks[1].clips.is_empty());
    let moved = store.project.tracks[2]
        .clips
        .iter()
        .find(|clip| clip.id == clip_id)
        .unwrap();
    assert_eq!(moved.start, 3.);
    assert_eq!(moved.offset, 1.);
    assert_eq!(moved.duration, 8.);
    commands::execute(&mut store, &json!({"command":"track_add"}))?;
    let added = store.project.tracks.last().unwrap();
    assert_eq!(added.kind, "audio");
    assert!(!added.armed);
    assert_eq!(
        store
            .project
            .tracks
            .iter()
            .filter(|track| track.kind == "voice")
            .count(),
        1
    );
    Ok(())
}

#[test]
fn invalid_batch_is_atomic_and_locked_tracks_can_only_be_copied() -> Result<()> {
    let (_directory, mut store) = setup()?;
    let selected = regions(&store, 3., 5.);
    store.project.tracks[2].locked = true;
    let original = serde_json::to_value(&store.project)?;
    for request in [
        json!({"command":"regions_edit","regions":selected,"delta":7.,"trackOffset":0}),
        json!({"command":"regions_edit","regions":selected,"delta":0.,"trackOffset":0,"remove":true}),
        json!({"command":"regions_edit","regions":[selected[0].clone()],"delta":-4.,"trackOffset":0}),
        json!({"command":"regions_edit","regions":[selected[0].clone()],"delta":0.,"trackOffset":-1}),
        json!({"command":"regions_edit","regions":[selected[0].clone(),selected[0].clone()],"delta":1.,"trackOffset":0}),
    ] {
        assert!(commands::execute(&mut store, &request).is_err());
        assert_eq!(serde_json::to_value(&store.project)?, original);
        assert!(store.history.is_empty());
    }
    Ok(())
}

#[test]
fn paste_preserves_offsets_spacing_and_track_mapping_and_rejects_other_projects() -> Result<()> {
    let (_directory, mut store) = setup()?;
    let mut clip = store.project.tracks[1].clips[0].clone();
    clip.start = 0.5;
    clip.offset = 2.;
    clip.duration = 1.25;
    let mut other = clip.clone();
    other.start = 2.;
    let request = json!({"command":"clips_paste","projectId":store.project.id,"trackId":store.project.tracks[1].id,"position":20.,"clips":[{"trackOffset":0,"clip":clip},{"trackOffset":1,"clip":other}]});
    let result = commands::execute(&mut store, &request)?;
    assert_eq!(store.history.len(), 1);
    assert_eq!(result["regions"][0]["from"], 20.5);
    assert_eq!(result["regions"][1]["from"], 22.);
    let pasted = store.project.tracks[1]
        .clips
        .iter()
        .find(|clip| clip.start == 20.5)
        .unwrap();
    assert_eq!(pasted.offset, 2.);
    assert_eq!(pasted.duration, 1.25);
    assert_ne!(pasted.id, clip.id);
    let before = serde_json::to_value(&store.project)?;
    let mut invalid = request.clone();
    invalid["projectId"] = json!(id());
    assert!(commands::execute(&mut store, &invalid).is_err());
    assert_eq!(serde_json::to_value(&store.project)?, before);
    invalid = request;
    invalid["clips"][1]["trackOffset"] = json!(128);
    assert!(commands::execute(&mut store, &invalid).is_err());
    assert_eq!(serde_json::to_value(&store.project)?, before);
    Ok(())
}

#[test]
fn section_removal_keeps_audio_outside_the_range() -> Result<()> {
    let (_directory, mut store) = setup()?;
    let selected = regions(&store, 3., 5.);
    commands::execute(
        &mut store,
        &json!({"command":"regions_edit","regions":selected,"delta":0.,"trackOffset":0,"remove":true}),
    )?;
    for track in &store.project.tracks[1..] {
        assert_eq!(track.clips.len(), 2);
        assert_eq!(track.clips[0].start + track.clips[0].duration, 3.);
        assert_eq!(track.clips[1].start, 5.);
    }
    Ok(())
}
