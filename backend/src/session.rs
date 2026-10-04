use crate::{
    dsp::Processor,
    media::{Peaks, finite, wav_header},
    mixer::Mixer,
    model::{Asset, Clip, MAX_UPLOAD, SAMPLE_RATE, id},
    storage::Store,
};
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    fs::File,
    io::{BufWriter, Read, Write},
};

pub struct Recording {
    file: BufWriter<File>,
    asset_id: String,
    track_id: String,
    start: f64,
    peaks: Peaks,
}
pub struct AudioSession {
    pub mixer: Option<Mixer>,
    pub frame: u64,
    pub playing: bool,
    pub monitor: bool,
    pub recording: Option<Recording>,
    pub input: VecDeque<[f32; 2]>,
    pub input_processor: Option<Processor>,
}
impl AudioSession {
    pub fn new() -> Self {
        Self {
            mixer: None,
            frame: 0,
            playing: false,
            monitor: false,
            recording: None,
            input: VecDeque::new(),
            input_processor: None,
        }
    }
    pub fn begin_record(&mut self, store: &mut Store, position: f64) -> Result<()> {
        ensure!(self.recording.is_none(), "Запись уже идёт");
        let track = store
            .project
            .tracks
            .iter()
            .find(|t| t.armed && t.kind != "video")
            .context("Включи R на звуковой дорожке")?
            .clone();
        self.mixer = None;
        store.materialize()?;
        let asset_id = id();
        let file = BufWriter::with_capacity(256 * 1024, File::create(store.pcm(&asset_id))?);
        self.recording = Some(Recording {
            file,
            asset_id,
            track_id: track.id,
            start: position,
            peaks: Peaks::new(),
        });
        self.input.clear();
        self.input_processor = Some(Processor::new(&track.effects));
        self.frame = (position * SAMPLE_RATE as f64).round() as u64;
        self.mixer = Some(Mixer::new(store)?);
        self.playing = true;
        Ok(())
    }
    pub fn finish_record(&mut self, store: &mut Store) -> Result<()> {
        self.playing = false;
        self.mixer = None;
        self.input.clear();
        let Some(mut recording) = self.recording.take() else {
            return Ok(());
        };
        recording.file.flush()?;
        recording.file.get_ref().sync_all()?;
        drop(recording.file);
        if recording.peaks.frames == 0 {
            return Ok(());
        }
        recording.peaks.finish();
        let frames = recording.peaks.frames;
        let mut source =
            BufWriter::with_capacity(256 * 1024, File::create(store.source(&recording.asset_id))?);
        source.write_all(&wav_header(frames))?;
        let mut pcm = File::open(store.pcm(&recording.asset_id))?;
        let mut bytes = vec![0u8; 8192 * 8];
        loop {
            let count = pcm.read(&mut bytes)?;
            if count == 0 {
                break;
            }
            for sample in bytes[..count].chunks_exact(4) {
                let x = f32::from_le_bytes(sample.try_into().unwrap());
                source.write_all(&((x.clamp(-1., 1.) * 32767.).round() as i16).to_le_bytes())?;
            }
        }
        source.flush()?;
        source.get_ref().sync_all()?;
        let name = format!(
            "Войс {}.wav",
            store
                .project
                .assets
                .iter()
                .filter(|a| a.name.starts_with("Войс "))
                .count()
                + 1
        );
        let asset = Asset {
            id: recording.asset_id.clone(),
            name: name.clone(),
            kind: "audio".into(),
            size: source.get_ref().metadata()?.len(),
            duration: frames as f64 / SAMPLE_RATE as f64,
            frames,
            sample_rate: SAMPLE_RATE,
            waveform: recording.peaks.bins,
            peak_frames: recording.peaks.width,
        };
        let mut project = store.project.clone();
        project
            .tracks
            .iter_mut()
            .find(|t| t.id == recording.track_id)
            .context("Нет дорожки записи")?
            .clips
            .push(Clip {
                id: id(),
                asset_id: asset.id.clone(),
                name,
                start: recording.start,
                offset: 0.,
                duration: asset.duration,
            });
        project.assets.push(asset);
        store.replace(project)
    }
    pub fn binary(&mut self, data: &[u8]) -> Result<Value> {
        if data == [2] {
            return Ok(json!({"type":"input-drained"}));
        }
        let (&kind, bytes) = data.split_first().context("Пустой пакет")?;
        if kind == 1 {
            ensure!(
                bytes.len() % 4 == 0 && bytes.len() <= 65536,
                "Некорректный пакет микрофона"
            );
            let recording = self.recording.as_mut().context("Запись не начата")?;
            let mut peak: f32 = 0.;
            for sample in bytes.chunks_exact(4) {
                let value = finite(f32::from_le_bytes(sample.try_into().unwrap()));
                peak = peak.max(value.abs());
                recording.file.write_all(&value.to_le_bytes())?;
                recording.file.write_all(&value.to_le_bytes())?;
                recording.peaks.push([value, value]);
                if self.monitor {
                    if self.input.len() >= 24000 {
                        self.input.pop_front();
                    }
                    self.input.push_back([value, value]);
                }
            }
            ensure!(
                recording.peaks.frames <= MAX_UPLOAD / 8,
                "Запись достигла лимита 100 ГБ"
            );
            Ok(json!({"type":"input","level":peak}))
        } else {
            bail!("Неизвестный бинарный пакет")
        }
    }
    pub fn pull(&mut self, store: &Store) -> Result<(Vec<u8>, Value)> {
        let count = 1024;
        ensure!(self.playing, "Воспроизведение остановлено");
        let project = &store.project;
        let (mut samples, mut meters) = self.mixer.as_mut().context("Нет проигрывателя")?.block(
            project,
            self.frame,
            count,
            None,
            self.recording.as_ref().map(|r| r.track_id.as_str()),
        )?;
        if let Some(recording) = &self.recording
            && self.monitor
        {
            let track = project
                .tracks
                .iter()
                .find(|t| t.id == recording.track_id)
                .unwrap();
            let mut input: Vec<_> = (0..count)
                .map(|_| self.input.pop_front().unwrap_or([0., 0.]))
                .collect();
            let level = self
                .input_processor
                .as_mut()
                .unwrap()
                .process(track, &mut input);
            meters.insert(track.id.clone(), level);
            if !track.mute && (!project.tracks.iter().any(|t| t.solo) || track.solo) {
                for (s, mic) in samples.iter_mut().zip(input) {
                    s[0] = (s[0] + mic[0]).clamp(-1., 1.);
                    s[1] = (s[1] + mic[1]).clamp(-1., 1.);
                }
            }
        }
        let mut packet = Vec::with_capacity(16 + count * 8);
        packet.extend(b"SVOP");
        packet.extend((count as u32).to_le_bytes());
        packet.extend((self.frame as f64 / SAMPLE_RATE as f64).to_le_bytes());
        for sample in samples {
            packet.extend(sample[0].to_le_bytes());
            packet.extend(sample[1].to_le_bytes());
        }
        self.frame += count as u64;
        let finished = self.recording.is_none()
            && self.frame as f64 / SAMPLE_RATE as f64 >= project.duration();
        if finished {
            self.playing = false;
        }
        Ok((
            packet,
            json!({"type":"meters","levels":meters,"finished":finished}),
        ))
    }
}
