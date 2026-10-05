use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const SAMPLE_RATE: u32 = 48_000;
pub const MAX_UPLOAD: u64 = 100_000_000_000;
pub const MAX_FRAMES: u64 = 2_000_000 * SAMPLE_RATE as u64;
pub fn id() -> String {
    Uuid::new_v4().to_string()
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Effects {
    pub highpass: f32,
    pub low_mid: f32,
    pub presence: f32,
    pub lowpass: f32,
    pub threshold: f32,
    pub ratio: f32,
    pub attack: f32,
    pub release: f32,
    pub makeup: f32,
    pub gate_threshold: f32,
    pub gate_reduction: f32,
    #[serde(default)]
    pub normalize: bool,
    #[serde(default = "default_loudness")]
    pub target_lufs: f32,
    #[serde(default = "default_true_peak")]
    pub true_peak: f32,
}
fn default_loudness() -> f32 {
    -16.
}
fn default_true_peak() -> f32 {
    -1.5
}
impl Effects {
    pub fn neutral() -> Self {
        Self {
            highpass: 20.,
            low_mid: 0.,
            presence: 0.,
            lowpass: 20000.,
            threshold: -24.,
            ratio: 1.,
            attack: 12.,
            release: 160.,
            makeup: 0.,
            gate_threshold: -48.,
            gate_reduction: 0.,
            normalize: false,
            target_lufs: default_loudness(),
            true_peak: default_true_peak(),
        }
    }
    pub fn voice() -> Self {
        Self {
            highpass: 70.,
            low_mid: -1.5,
            presence: 1.,
            lowpass: 15000.,
            ratio: 2.3,
            makeup: 2.9,
            normalize: true,
            ..Self::neutral()
        }
    }
    pub fn validate(&self) -> Result<()> {
        for (v, lo, hi) in [
            (self.highpass, 20., 180.),
            (self.low_mid, -6., 6.),
            (self.presence, -6., 6.),
            (self.lowpass, 6000., 20000.),
            (self.threshold, -48., 0.),
            (self.ratio, 1., 8.),
            (self.attack, 1., 80.),
            (self.release, 50., 500.),
            (self.makeup, 0., 8.),
            (self.gate_threshold, -70., -25.),
            (self.gate_reduction, 0., 12.),
            (self.target_lufs, -24., -9.),
            (self.true_peak, -6., -0.1),
        ] {
            ensure!(
                v.is_finite() && v >= lo && v <= hi,
                crate::i18n::message("error.effectValue")
            );
        }
        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Clip {
    pub id: String,
    pub asset_id: String,
    pub name: String,
    pub start: f64,
    pub offset: f64,
    pub duration: f64,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub color: String,
    pub mute: bool,
    pub solo: bool,
    pub locked: bool,
    pub armed: bool,
    pub fx_bypass: bool,
    pub volume: f32,
    pub pan: f32,
    pub effects: Effects,
    pub clips: Vec<Clip>,
}
impl Track {
    pub fn new(kind: &str, name: &str) -> Self {
        Self {
            id: id(),
            name: name.into(),
            kind: kind.into(),
            color: match kind {
                "video" => "#a5bbeb",
                "voice" => "#d6ee9b",
                _ => "#dab9e9",
            }
            .into(),
            mute: false,
            solo: false,
            locked: false,
            armed: kind == "voice",
            fx_bypass: kind != "voice",
            volume: 0.,
            pan: 0.,
            effects: if kind == "voice" {
                Effects::voice()
            } else {
                Effects::neutral()
            },
            clips: vec![],
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub size: u64,
    pub duration: f64,
    pub frames: u64,
    pub sample_rate: u32,
    pub waveform: Vec<[f32; 2]>,
    pub peak_frames: u64,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub version: u32,
    pub id: String,
    pub name: String,
    pub tracks: Vec<Track>,
    pub assets: Vec<Asset>,
}
impl Project {
    pub fn new() -> Self {
        Self {
            version: 3,
            id: id(),
            name: "project.defaultName".into(),
            tracks: vec![
                Track::new("video", "track.video"),
                Track::new("voice", "track.voice"),
                Track::new("audio", "track.music"),
            ],
            assets: vec![],
        }
    }
    pub fn duration(&self) -> f64 {
        self.tracks
            .iter()
            .flat_map(|t| &t.clips)
            .map(|c| c.start + c.duration)
            .fold(0., f64::max)
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            [2, 3].contains(&self.version),
            crate::i18n::message("error.projectVersion")
        );
        Uuid::parse_str(&self.id)?;
        ensure!(
            self.tracks.len() <= 128 && self.assets.len() <= 10000,
            crate::i18n::message("error.projectItemCount")
        );
        let mut ids = std::collections::HashSet::new();
        let mut armed = 0;
        for a in &self.assets {
            Uuid::parse_str(&a.id)?;
            ensure!(
                ids.insert(a.id.clone()),
                crate::i18n::message("error.duplicateId")
            );
            ensure!(
                a.size <= MAX_UPLOAD
                    && a.duration.is_finite()
                    && a.duration >= 0.
                    && a.frames <= MAX_FRAMES
                    && a.sample_rate == SAMPLE_RATE,
                crate::i18n::message("error.media")
            );
            ensure!(
                a.peak_frames > 0
                    && a.waveform.len() <= 65536
                    && a.waveform.iter().flatten().all(|x| x.is_finite()),
                crate::i18n::message("error.waveform")
            );
        }
        for t in &self.tracks {
            Uuid::parse_str(&t.id)?;
            ensure!(
                ids.insert(t.id.clone()),
                crate::i18n::message("error.duplicateId")
            );
            ensure!(
                ["audio", "video", "voice"].contains(&t.kind.as_str()),
                crate::i18n::message("error.trackType")
            );
            ensure!(
                t.volume.is_finite()
                    && (-60. ..=12.).contains(&t.volume)
                    && t.pan.is_finite()
                    && (-1. ..=1.).contains(&t.pan),
                crate::i18n::message("error.mixer")
            );
            ensure!(
                t.clips.len() <= 10000 && t.name.len() <= 512,
                crate::i18n::message("error.clipCount")
            );
            t.effects.validate()?;
            armed += usize::from(t.armed);
            ensure!(
                !t.armed || t.kind != "video",
                crate::i18n::message("error.trackCannotRecord")
            );
            for c in &t.clips {
                Uuid::parse_str(&c.id)?;
                ensure!(
                    ids.insert(c.id.clone()),
                    crate::i18n::message("error.duplicateId")
                );
                let a = self
                    .assets
                    .iter()
                    .find(|a| a.id == c.asset_id)
                    .ok_or_else(|| {
                        anyhow::anyhow!(crate::i18n::message("error.clipSourceMissing"))
                    })?;
                ensure!(
                    [c.start, c.offset, c.duration]
                        .iter()
                        .all(|x| x.is_finite() && *x >= 0.)
                        && c.duration > 0.
                        && c.offset + c.duration <= a.duration + 0.001
                        && c.start <= 2_000_000.,
                    crate::i18n::message("error.clipBounds")
                );
            }
        }
        ensure!(
            armed <= 1,
            crate::i18n::message("error.recordingTrackCount")
        );
        Ok(())
    }
}
