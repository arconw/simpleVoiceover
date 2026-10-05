use crate::{
    mixer::Mixer,
    model::{SAMPLE_RATE, Track},
    storage::Store,
};
use anyhow::Result;
use ebur128::{EbuR128, Mode};
use std::{
    collections::{HashMap, VecDeque},
    hash::{Hash, Hasher},
};

#[derive(Clone)]
pub struct Analysis {
    pub loudness: f64,
    pub true_peak: f64,
}

pub type LoudnessCache = VecDeque<(u64, Analysis)>;

fn fingerprint(track: &Track) -> Result<u64> {
    let mut effects = track.effects.clone();
    effects.normalize = false;
    effects.target_lufs = -16.;
    effects.true_peak = -1.5;
    let origin = track
        .clips
        .iter()
        .map(|clip| (clip.start * SAMPLE_RATE as f64).round() as u64)
        .min()
        .unwrap_or(0);
    let clips: Vec<_> = track
        .clips
        .iter()
        .map(|clip| {
            (
                &clip.asset_id,
                (clip.start * SAMPLE_RATE as f64).round() as u64 - origin,
                (clip.offset * SAMPLE_RATE as f64).round() as u64,
                (clip.duration * SAMPLE_RATE as f64).round() as u64,
            )
        })
        .collect();
    let bytes = serde_json::to_vec(&(effects, track.fx_bypass, clips))?;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    Ok(hasher.finish())
}

pub fn gain(analysis: &Analysis, track: &Track) -> f32 {
    if !analysis.loudness.is_finite() || analysis.true_peak <= 1e-12 {
        return 1.;
    }
    let desired = track.effects.target_lufs as f64 - analysis.loudness;
    let ceiling = track.effects.true_peak as f64 - 20. * analysis.true_peak.log10();
    10f64.powf(desired.min(ceiling).min(30.) / 20.) as f32
}

pub fn measure(
    store: &Store,
    track: &Track,
    progress: &crate::progress::Progress,
) -> Result<Analysis> {
    let mut project = store.project.clone();
    let measured = project
        .tracks
        .iter_mut()
        .find(|item| item.id == track.id)
        .unwrap();
    measured.volume = 0.;
    measured.pan = 0.;
    measured.effects.normalize = false;
    let start = track
        .clips
        .iter()
        .map(|clip| (clip.start * SAMPLE_RATE as f64).round() as u64)
        .min()
        .unwrap_or(0);
    let end = track
        .clips
        .iter()
        .map(|clip| ((clip.start + clip.duration) * SAMPLE_RATE as f64).ceil() as u64)
        .max()
        .unwrap_or(start);
    let total = end.saturating_sub(start);
    let mut meter = EbuR128::new(2, SAMPLE_RATE, Mode::I | Mode::TRUE_PEAK | Mode::HISTOGRAM)?;
    let mut mixer = Mixer::unscaled(store)?;
    let label = crate::i18n::message("progress.loudness");
    progress.report(&label, 0.);
    let mut frame = start;
    let mut interleaved = Vec::with_capacity(8192);
    while frame < end {
        let count = (end - frame).min(4096) as usize;
        let (samples, _) = mixer.block(&project, frame, count, Some(&track.id), None)?;
        interleaved.clear();
        interleaved.extend(samples.into_iter().flatten());
        meter.add_frames_f32(&interleaved)?;
        frame += count as u64;
        progress.report(&label, (frame - start) as f64 / total.max(1) as f64);
    }
    let padding = (SAMPLE_RATE as u64 * 2 / 5).saturating_sub(total).max(512) as usize;
    meter.add_frames_f32(&vec![0.; padding * 2])?;
    progress.report(&label, 1.);
    Ok(Analysis {
        loudness: meter.loudness_global()?,
        true_peak: meter.true_peak(0)?.max(meter.true_peak(1)?),
    })
}

pub fn gains(
    store: &Store,
    selected: Option<&str>,
    suppressed: Option<&str>,
) -> Result<HashMap<String, (crate::model::Effects, f32)>> {
    let solo = store.project.tracks.iter().any(|track| track.solo);
    let tracks: Vec<_> = store
        .project
        .tracks
        .iter()
        .filter(|track| {
            !track.fx_bypass
                && track.effects.normalize
                && !track.clips.is_empty()
                && !suppressed.is_some_and(|id| id == track.id)
                && if let Some(id) = selected {
                    id == track.id
                } else {
                    !track.mute && (!solo || track.solo)
                }
        })
        .collect();
    let mut gains = HashMap::new();
    for (index, track) in tracks.iter().enumerate() {
        let key = fingerprint(track)?;
        let cached = store
            .loudness_cache
            .lock()
            .unwrap()
            .iter()
            .find(|(id, _)| *id == key)
            .map(|(_, analysis)| analysis.clone());
        let analysis = if let Some(analysis) = cached {
            analysis
        } else {
            let progress = store
                .progress
                .range(index as f64 / tracks.len() as f64, 1. / tracks.len() as f64);
            let analysis = measure(store, track, &progress)?;
            let mut cache = store.loudness_cache.lock().unwrap();
            if cache.len() >= 128 {
                cache.pop_front();
            }
            cache.push_back((key, analysis.clone()));
            analysis
        };
        gains.insert(
            track.id.clone(),
            (track.effects.clone(), gain(&analysis, track)),
        );
    }
    Ok(gains)
}
