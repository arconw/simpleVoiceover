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
        ] {
            ensure!(
                v.is_finite() && v >= lo && v <= hi,
                "Недопустимое значение эффекта"
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
            version: 2,
            id: id(),
            name: "Моя сессия".into(),
            tracks: vec![
                Track::new("video", "Звук видео"),
                Track::new("voice", "Мой голос"),
                Track::new("audio", "Музыка и фон"),
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
        ensure!(self.version == 2, "Неподдерживаемая версия проекта");
        Uuid::parse_str(&self.id)?;
        ensure!(
            self.tracks.len() <= 128 && self.assets.len() <= 10000,
            "Слишком много дорожек или медиа"
        );
        let mut ids = std::collections::HashSet::new();
        let mut armed = 0;
        for a in &self.assets {
            Uuid::parse_str(&a.id)?;
            ensure!(ids.insert(a.id.clone()), "Повторяющийся идентификатор");
            ensure!(
                a.size <= MAX_UPLOAD
                    && a.duration.is_finite()
                    && a.duration >= 0.
                    && a.frames <= MAX_FRAMES
                    && a.sample_rate == SAMPLE_RATE,
                "Некорректное медиа"
            );
            ensure!(
                a.peak_frames > 0
                    && a.waveform.len() <= 65536
                    && a.waveform.iter().flatten().all(|x| x.is_finite()),
                "Некорректная форма волны"
            );
        }
        for t in &self.tracks {
            Uuid::parse_str(&t.id)?;
            ensure!(ids.insert(t.id.clone()), "Повторяющийся идентификатор");
            ensure!(
                ["audio", "video", "voice"].contains(&t.kind.as_str()),
                "Неизвестный тип дорожки"
            );
            ensure!(
                t.volume.is_finite()
                    && (-60. ..=12.).contains(&t.volume)
                    && t.pan.is_finite()
                    && (-1. ..=1.).contains(&t.pan),
                "Некорректный микшер"
            );
            ensure!(
                t.clips.len() <= 10000 && t.name.len() <= 512,
                "Слишком много клипов"
            );
            t.effects.validate()?;
            armed += usize::from(t.armed);
            ensure!(
                !t.armed || t.kind != "video",
                "Эта дорожка не может записывать"
            );
            for c in &t.clips {
                Uuid::parse_str(&c.id)?;
                ensure!(ids.insert(c.id.clone()), "Повторяющийся идентификатор");
                let a = self
                    .assets
                    .iter()
                    .find(|a| a.id == c.asset_id)
                    .ok_or_else(|| anyhow::anyhow!("Нет исходника клипа"))?;
                ensure!(
                    [c.start, c.offset, c.duration]
                        .iter()
                        .all(|x| x.is_finite() && *x >= 0.)
                        && c.duration > 0.
                        && c.offset + c.duration <= a.duration + 0.001
                        && c.start <= 2_000_000.,
                    "Некорректные границы клипа"
                );
            }
        }
        ensure!(armed <= 1, "Одновременно может записываться одна дорожка");
        Ok(())
    }
}
