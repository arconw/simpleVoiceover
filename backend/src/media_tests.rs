use crate::{
    commands, media, mixer,
    model::{Asset, SAMPLE_RATE, id},
    storage::Store,
};
use anyhow::Result;
use serde_json::json;
use std::{fs, path::Path};

fn element(identifier: &[u8], payload: &[u8]) -> Vec<u8> {
    let mut result = identifier.to_vec();
    let width = (1..=8)
        .find(|width| (payload.len() as u64) < (1u64 << (7 * width)) - 1)
        .unwrap();
    let size = (payload.len() as u64 | (1u64 << (7 * width))).to_be_bytes();
    result.extend_from_slice(&size[8 - width..]);
    result.extend_from_slice(payload);
    result
}

fn integer(identifier: &[u8], value: u64) -> Vec<u8> {
    element(identifier, &value.to_be_bytes())
}

fn matroska(path: &Path, unsupported: bool, silent: bool) -> Result<()> {
    matroska_packets(path, unsupported, silent, 480)
}

fn matroska_packets(
    path: &Path,
    unsupported: bool,
    silent: bool,
    packet_frames: u64,
) -> Result<()> {
    let mut header = element(&[0x42, 0x82], b"matroska");
    header.extend(integer(&[0x42, 0x87], 4));
    let mut bytes = element(&[0x1a, 0x45, 0xdf, 0xa3], &header);
    let mut info = integer(&[0x2a, 0xd7, 0xb1], 1_000_000);
    info.extend(element(&[0x44, 0x89], &2000f64.to_be_bytes()));
    let mut segment = element(&[0x15, 0x49, 0xa9, 0x66], &info);
    let mut video = integer(&[0xd7], 1);
    video.extend(integer(&[0x73, 0xc5], 1));
    video.extend(integer(&[0x83], 1));
    video.extend(element(&[0x86], b"V_MPEG4/ISO/AVC"));
    let mut tracks = element(&[0xae], &video);
    if !silent {
        for index in 0..3 {
            let mut track = integer(&[0xd7], index + 2);
            track.extend(integer(&[0x73, 0xc5], index + 2));
            track.extend(integer(&[0x83], 2));
            track.extend(element(
                &[0x86],
                if unsupported && index == 1 {
                    b"A_AC3"
                } else {
                    b"A_PCM/INT/LIT"
                },
            ));
            track.extend(element(
                &[0x22, 0xb5, 0x9c],
                [b"eng", b"rus", b"deu"][index as usize],
            ));
            let mut audio = element(&[0xb5], &48000f64.to_be_bytes());
            audio.extend(integer(&[0x9f], 1));
            audio.extend(integer(&[0x62, 0x64], 16));
            track.extend(element(&[0xe1], &audio));
            tracks.extend(element(&[0xae], &track));
        }
    }
    segment.extend(element(&[0x16, 0x54, 0xae, 0x6b], &tracks));
    if !silent {
        for packet in 0..96000 / packet_frames {
            let start = packet * packet_frames;
            let mut cluster = integer(&[0xe7], start * 1000 / SAMPLE_RATE as u64);
            for (index, frequency) in [440., 660., 880.].into_iter().enumerate() {
                if index == 2 && start < 24000 && packet_frames < 24000 {
                    continue;
                }
                let mut block = vec![0x82 + index as u8, 0, 0, 0x80];
                for frame in 0..packet_frames {
                    let sample = (0.2
                        * (std::f64::consts::TAU * frequency * (start + frame) as f64
                            / SAMPLE_RATE as f64)
                            .sin()
                        * i16::MAX as f64) as i16;
                    block.extend(sample.to_le_bytes());
                }
                cluster.extend(element(&[0xa3], &block));
            }
            segment.extend(element(&[0x1f, 0x43, 0xb6, 0x75], &cluster));
        }
    }
    bytes.extend(element(&[0x18, 0x53, 0x80, 0x67], &segment));
    fs::write(path, bytes)?;
    Ok(())
}

#[test]
fn matroska_large_pcm_packets_are_decoded_without_truncation() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let source = directory.path().join("large-packets.mkv");
    matroska_packets(&source, false, false, 96000)?;
    let mut store = Store::open(
        directory.path().join("config"),
        Some(directory.path().join("work")),
    )?;
    commands::import_source(
        &mut store,
        source,
        "large-packets.mkv".into(),
        "video".into(),
        None,
        0.,
    )?;
    for asset in store
        .project
        .assets
        .iter()
        .filter(|asset| asset.kind == "audio")
    {
        assert_eq!(asset.frames, 96000);
        assert_eq!(fs::metadata(store.pcm(&asset.id))?.len(), 96000 * 8);
    }
    Ok(())
}

#[test]
fn matroska_preview_keeps_video_and_disables_all_audio_without_modifying_source_bytes() -> Result<()>
{
    let directory = tempfile::tempdir()?;
    let source = directory.path().join("video.mkv");
    matroska(&source, false, false)?;
    let original = fs::read(&source)?;
    let patches = crate::video_source::patches(&source, 0, original.len() as u64, "video.mkv")?;
    assert_eq!(patches.len(), 3);
    let mut preview = original.clone();
    crate::video_source::apply(&patches, 0, &mut preview);
    let preview_path = directory.path().join("preview.mkv");
    fs::write(&preview_path, &preview)?;
    let stream = symphonia::core::io::MediaSourceStream::new(
        Box::new(fs::File::open(preview_path)?),
        Default::default(),
    );
    let probe = symphonia::default::get_probe().format(
        &Default::default(),
        stream,
        &Default::default(),
        &Default::default(),
    )?;
    assert_eq!(probe.format.tracks().len(), 1);
    assert!(probe.format.tracks()[0].codec_params.sample_rate.is_none());
    assert_eq!(preview.len(), original.len());
    assert_eq!(fs::read(source)?, original);
    Ok(())
}

#[test]
fn matroska_streams_keep_voice_track_order_independent_exports_and_project_history() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let source = directory.path().join("procedural.mkv");
    matroska(&source, false, false)?;
    let mut store = Store::open(
        directory.path().join("config"),
        Some(directory.path().join("work")),
    )?;
    let voice_id = store.project.tracks[1].id.clone();
    let original = serde_json::to_value(&store.project)?;
    commands::import_source(
        &mut store,
        source.clone(),
        "procedural.mkv".into(),
        "video".into(),
        None,
        1.,
    )?;
    assert_eq!(store.project.assets.len(), 4);
    assert_eq!(store.project.tracks.len(), 6);
    assert_eq!(store.project.tracks[4].id, voice_id);
    assert!(store.project.tracks[4].armed);
    assert_eq!(
        fs::read(store.source(&store.project.assets[0].id))?,
        fs::read(&source)?
    );
    assert_eq!(store.project.assets[0].frames, 0);
    for index in 1..=3 {
        let track = &store.project.tracks[index];
        assert_eq!(track.kind, "audio");
        assert_eq!(
            track.name,
            format!("procedural [{index}] {}", ["eng", "rus", "deu"][index - 1])
        );
        assert_eq!(track.mute, index > 1);
        assert_eq!(track.clips[0].start, 1.);
        assert!((track.clips[0].duration - 2.).abs() < 0.001);
        assert_eq!(
            fs::metadata(store.source(&store.project.assets[index].id))?.len(),
            store.project.assets[index].size
        );
        let export = directory.path().join(format!("stream-{index}.wav"));
        mixer::render(&store, &export, Some(&track.id), "wav")?;
        let pcm = fs::read(store.pcm(&store.project.assets[index].id))?;
        assert!(
            pcm.chunks_exact(8)
                .any(|frame| f32::from_le_bytes(frame[..4].try_into().unwrap()).abs() > 0.1)
        );
        if index == 3 {
            assert!(pcm[..24000 * 8].iter().all(|value| *value == 0));
        }
    }
    let imported = serde_json::to_value(&store.project)?;
    commands::execute(&mut store, &json!({"command":"undo"}))?;
    assert_eq!(serde_json::to_value(&store.project)?, original);
    commands::execute(&mut store, &json!({"command":"redo"}))?;
    assert_eq!(serde_json::to_value(&store.project)?, imported);
    let project = directory.path().join("streams.svoice");
    store.save(&project)?;
    store.load(&project)?;
    let reopened = serde_json::to_value(&store.project)?;
    assert_ne!(reopened["id"], imported["id"]);
    for key in ["name", "version", "tracks"] {
        assert_eq!(reopened[key], imported[key], "project {key}");
    }
    for (index, asset) in reopened["assets"].as_array().unwrap().iter().enumerate() {
        for (key, value) in asset.as_object().unwrap() {
            assert!(
                value == &imported["assets"][index][key],
                "asset {index} field {key} differs"
            );
        }
    }
    for index in 1..=3 {
        let track = &store.project.tracks[index];
        mixer::render(
            &store,
            &directory.path().join(format!("reopened-{index}.mp3")),
            Some(&track.id),
            "mp3",
        )?;
    }
    Ok(())
}

#[test]
fn unsupported_matroska_stream_rolls_back_files_project_and_dirty_state() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let source = directory.path().join("unsupported.mkv");
    matroska(&source, true, false)?;
    let mut store = Store::open(
        directory.path().join("config"),
        Some(directory.path().join("work")),
    )?;
    let saved = directory.path().join("saved.svoice");
    store.save(&saved)?;
    let original = fs::read(&saved)?;
    assert!(
        commands::import_source(
            &mut store,
            source,
            "unsupported.mkv".into(),
            "video".into(),
            None,
            0.
        )
        .is_err()
    );
    assert!(!store.config.dirty);
    assert!(store.project.assets.is_empty());
    assert!(store.history.is_empty());
    assert!(!store.root().exists());
    assert_eq!(fs::read(saved)?, original);
    Ok(())
}

#[test]
fn matroska_without_audio_has_a_video_clip_and_silent_export() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let source = directory.path().join("silent.mkv");
    matroska(&source, false, true)?;
    let mut store = Store::open(
        directory.path().join("config"),
        Some(directory.path().join("work")),
    )?;
    commands::import_source(
        &mut store,
        source,
        "silent.mkv".into(),
        "video".into(),
        None,
        0.,
    )?;
    assert_eq!(store.project.assets.len(), 1);
    assert_eq!(store.project.duration(), 2.);
    let mut renderer = mixer::Mixer::new(&store)?;
    let (samples, _) = renderer.block(&store.project, 0, 2048, None, None)?;
    assert!(samples.iter().flatten().all(|sample| *sample == 0.));
    let export = directory.path().join("silent.wav");
    mixer::render(&store, &export, None, "wav")?;
    let asset = Asset {
        id: id(),
        name: "silent.wav".into(),
        kind: "audio".into(),
        size: fs::metadata(&export)?.len(),
        duration: 0.,
        frames: 0,
        sample_rate: SAMPLE_RATE,
        waveform: vec![],
        peak_frames: 256,
    };
    assert_eq!(
        media::decode(&export, &directory.path().join("silent.pcm"), asset)?.duration,
        2.
    );
    Ok(())
}
