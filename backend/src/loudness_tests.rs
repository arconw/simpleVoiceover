use crate::{
    commands, loudness,
    media::wav_header,
    mixer::Mixer,
    model::{Effects, SAMPLE_RATE},
    storage::Store,
};
use anyhow::Result;
use ebur128::{EbuR128, Mode};
use serde_json::json;
use std::{
    fs::{self, File},
    io::Write,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

fn signal_store(
    duration: f64,
    amplitude: f32,
    transient: bool,
) -> Result<(tempfile::TempDir, Store)> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("input.wav");
    let frames = (duration * SAMPLE_RATE as f64) as u64;
    let mut file = File::create(&path)?;
    file.write_all(&wav_header(frames))?;
    for frame in 0..frames {
        let sample = if transient && frame == frames / 2 {
            0.95
        } else {
            amplitude
                * (2. * std::f32::consts::PI * 1000. * frame as f32 / SAMPLE_RATE as f32).sin()
        };
        let pcm = (sample * 32767.).round() as i16;
        file.write_all(&pcm.to_le_bytes())?;
        file.write_all(&pcm.to_le_bytes())?;
    }
    drop(file);
    let mut store = Store::open(
        directory.path().join("config"),
        Some(directory.path().join("work")),
    )?;
    let track = store.project.tracks[1].id.clone();
    commands::import_source(
        &mut store,
        path,
        "voice.wav".into(),
        "audio".into(),
        Some(track.clone()),
        0.,
    )?;
    let mut effects = Effects::neutral();
    effects.normalize = true;
    commands::execute(
        &mut store,
        &json!({"command":"track_patch","trackId":track,"patch":{"effects":effects,"fxBypass":false}}),
    )?;
    Ok((directory, store))
}

fn output_meter(store: &Store) -> Result<EbuR128> {
    let mut mixer = Mixer::new(store)?;
    let mut meter = EbuR128::new(2, SAMPLE_RATE, Mode::I | Mode::TRUE_PEAK)?;
    let total = (store.project.duration() * SAMPLE_RATE as f64).ceil() as u64;
    let mut frame = 0;
    while frame < total {
        let count = (total - frame).min(4096) as usize;
        let (samples, _) = mixer.block(&store.project, frame, count, None, None)?;
        meter.add_frames_f32(&samples.into_iter().flatten().collect::<Vec<_>>())?;
        frame += count as u64;
    }
    meter.add_frames_f32(&[0.; 1024])?;
    Ok(meter)
}

#[test]
fn normalization_reaches_loudness_target_and_keeps_original_samples() -> Result<()> {
    let (_directory, store) = signal_store(3., 0.035, false)?;
    let original = fs::read(store.pcm(&store.project.assets[0].id))?;
    let meter = output_meter(&store)?;
    assert!((meter.loudness_global()? + 16.).abs() < 0.15);
    assert!(20. * meter.true_peak(0)?.log10() <= -1.49);
    assert_eq!(fs::read(store.pcm(&store.project.assets[0].id))?, original);
    Ok(())
}

#[test]
fn transient_peak_ceiling_takes_priority_over_average_loudness() -> Result<()> {
    let (_directory, mut store) = signal_store(3., 0.012, true)?;
    store.project.tracks[1].effects.true_peak = -3.;
    store.project.tracks[1].effects.target_lufs = -9.;
    let meter = output_meter(&store)?;
    assert!(20. * meter.true_peak(0)?.log10() <= -2.99);
    assert!(meter.loudness_global()? < -10.);
    Ok(())
}

#[test]
fn loudness_cache_reuses_measurements_but_remeasures_changed_processing() -> Result<()> {
    let (_directory, mut store) = signal_store(1., 0.035, false)?;
    let events = Arc::new(AtomicUsize::new(0));
    let counter = events.clone();
    store.progress = crate::progress::Progress::new(move |_| {
        counter.fetch_add(1, Ordering::SeqCst);
    });
    let _ = Mixer::new(&store)?;
    let measured = events.load(Ordering::SeqCst);
    assert!(measured > 0);
    store.project.tracks[1].volume = -3.;
    store.project.tracks[1].effects.target_lufs = -18.;
    let _ = Mixer::new(&store)?;
    assert_eq!(events.load(Ordering::SeqCst), measured);
    for clip in &mut store.project.tracks[1].clips {
        clip.start += 5.;
    }
    let _ = Mixer::new(&store)?;
    assert_eq!(events.load(Ordering::SeqCst), measured);
    store.project.tracks[1].effects.highpass = 100.;
    let _ = Mixer::new(&store)?;
    assert!(events.load(Ordering::SeqCst) > measured);
    Ok(())
}

#[test]
fn silence_short_takes_bypass_and_legacy_effects_remain_valid() -> Result<()> {
    for (duration, amplitude) in [(0.05, 0.04), (0.5, 0.)] {
        let (_directory, mut store) = signal_store(duration, amplitude, false)?;
        let meter = output_meter(&store)?;
        assert!(meter.true_peak(0)?.is_finite());
        if amplitude == 0. {
            assert_eq!(meter.true_peak(0)?, 0.);
        }
        store.project.tracks[1].fx_bypass = true;
        let _ = Mixer::new(&store)?;
    }
    let mut old = serde_json::to_value(Effects::voice())?;
    for key in ["normalize", "targetLufs", "truePeak"] {
        old.as_object_mut().unwrap().remove(key);
    }
    let effects: Effects = serde_json::from_value(old)?;
    assert!(!effects.normalize);
    assert_eq!(effects.target_lufs, -16.);
    assert_eq!(effects.true_peak, -1.5);
    effects.validate()?;
    let analysis = loudness::Analysis {
        loudness: f64::NEG_INFINITY,
        true_peak: 0.,
    };
    assert_eq!(loudness::gain(&analysis, &store_track()), 1.);
    Ok(())
}

fn store_track() -> crate::model::Track {
    crate::model::Track::new("voice", "Voice")
}

#[test]
#[ignore = "Measures a WAV supplied through VOICEOVER_REFERENCE_WAV"]
fn voice_preset_reference_comparison() -> Result<()> {
    let source = std::env::var("VOICEOVER_REFERENCE_WAV")?;
    let directory = tempfile::tempdir()?;
    let mut store = Store::open(
        directory.path().join("config"),
        Some(directory.path().join("work")),
    )?;
    let track_id = store.project.tracks[1].id.clone();
    commands::import_source(
        &mut store,
        source.into(),
        "reference.wav".into(),
        "audio".into(),
        Some(track_id.clone()),
        0.,
    )?;
    store.project.tracks[1].fx_bypass = true;
    let original = loudness::measure(&store, &store.project.tracks[1], &store.progress)?;
    store.project.tracks[1].fx_bypass = false;
    let processed = loudness::measure(&store, &store.project.tracks[1], &store.progress)?;
    let meter = output_meter(&store)?;
    println!(
        "original_stereo_lufs={:.2} eq_rms_stereo_lufs={:.2} normalized_stereo_lufs={:.2} normalized_dbtp={:.2} gain_db={:.2}",
        original.loudness,
        processed.loudness,
        meter.loudness_global()?,
        20. * meter.true_peak(0)?.log10(),
        20. * loudness::gain(&processed, &store.project.tracks[1]).log10()
    );
    Ok(())
}
