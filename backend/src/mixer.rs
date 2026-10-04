use crate::{
    dsp::Processor,
    media::wav_header,
    model::{Project, SAMPLE_RATE},
    storage::Store,
};
use anyhow::{Result, ensure};
use shine_rs::{Mp3Encoder, Mp3EncoderConfig};
use std::{
    collections::HashMap,
    fs::File,
    io::{BufWriter, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

pub struct Mixer {
    readers: HashMap<String, (File, u64, u64)>,
    processors: HashMap<String, Processor>,
    locations: HashMap<String, (PathBuf, u64, u64)>,
}
type StereoBlock = (Vec<[f32; 2]>, HashMap<String, f32>);
impl Mixer {
    pub fn new(store: &Store) -> Result<Self> {
        let mut locations = HashMap::new();
        for asset in &store.project.assets {
            locations.insert(asset.id.clone(), store.location(&asset.id, true)?);
        }
        Ok(Self {
            readers: HashMap::new(),
            processors: HashMap::new(),
            locations,
        })
    }
    pub fn block(
        &mut self,
        project: &Project,
        start: u64,
        frames: usize,
        selected: Option<&str>,
        suppressed: Option<&str>,
    ) -> Result<StereoBlock> {
        let mut mix = vec![[0.; 2]; frames];
        let mut meters = HashMap::new();
        let solo = project.tracks.iter().any(|t| t.solo);
        for track in &project.tracks {
            if selected.is_some_and(|id| id != track.id)
                || selected.is_none() && (track.mute || (solo && !track.solo))
                || suppressed.is_some_and(|id| id == track.id)
            {
                continue;
            }
            let mut samples = vec![[0.; 2]; frames];
            for clip in &track.clips {
                let clip_start = (clip.start * SAMPLE_RATE as f64).round() as u64;
                let clip_end = clip_start + (clip.duration * SAMPLE_RATE as f64).round() as u64;
                let from = start.max(clip_start);
                let to = (start + frames as u64).min(clip_end);
                if from >= to {
                    continue;
                }
                if !self.readers.contains_key(&clip.asset_id) {
                    let (path, offset, length) = self
                        .locations
                        .get(&clip.asset_id)
                        .ok_or_else(|| anyhow::anyhow!("Нет кэша исходника"))?;
                    self.readers
                        .insert(clip.asset_id.clone(), (File::open(path)?, *offset, *length));
                }
                let (file, base, length) = self.readers.get_mut(&clip.asset_id).unwrap();
                let source = (clip.offset * SAMPLE_RATE as f64).round() as u64 + from - clip_start;
                let available = (*length / 8).saturating_sub(source).min(to - from) as usize;
                if available == 0 {
                    continue;
                }
                file.seek(SeekFrom::Start(*base + source * 8))?;
                let mut bytes = vec![0u8; available * 8];
                file.read_exact(&mut bytes)?;
                for (index, bytes) in bytes.chunks_exact(8).enumerate() {
                    let dest = &mut samples[(from - start) as usize + index];
                    dest[0] += f32::from_le_bytes(bytes[..4].try_into().unwrap());
                    dest[1] += f32::from_le_bytes(bytes[4..].try_into().unwrap());
                }
            }
            let processor = self
                .processors
                .entry(track.id.clone())
                .or_insert_with(|| Processor::new(&track.effects));
            meters.insert(track.id.clone(), processor.process(track, &mut samples));
            for (mixed, sample) in mix.iter_mut().zip(samples) {
                mixed[0] += sample[0];
                mixed[1] += sample[1];
            }
        }
        for sample in &mut mix {
            sample[0] = sample[0].clamp(-1., 1.);
            sample[1] = sample[1].clamp(-1., 1.);
        }
        Ok((mix, meters))
    }
}

pub fn render(
    store: &Store,
    destination: &Path,
    selected: Option<&str>,
    format: &str,
) -> Result<()> {
    ensure!(["wav", "mp3"].contains(&format), "Неподдерживаемый формат");
    let duration = if let Some(id) = selected {
        store
            .project
            .tracks
            .iter()
            .find(|t| t.id == id)
            .ok_or_else(|| anyhow::anyhow!("Нет дорожки"))?
            .clips
            .iter()
            .map(|c| c.start + c.duration)
            .fold(0., f64::max)
    } else {
        store.project.duration()
    };
    ensure!(duration > 0., "Нет аудио для экспорта");
    let total = (duration * SAMPLE_RATE as f64).ceil() as u64;
    let mut mixer = Mixer::new(store)?;
    let mut writer = BufWriter::new(File::create(destination)?);
    let mut encoder = if format == "mp3" {
        Some(Mp3Encoder::new(
            Mp3EncoderConfig::new()
                .sample_rate(SAMPLE_RATE)
                .bitrate(256)
                .channels(2),
        )?)
    } else {
        writer.write_all(&wav_header(total))?;
        None
    };
    let mut frame = 0;
    while frame < total {
        let count = (total - frame).min(4096) as usize;
        let (samples, _) = mixer.block(&store.project, frame, count, selected, None)?;
        let pcm: Vec<i16> = samples
            .iter()
            .flatten()
            .map(|v| (v.clamp(-1., 1.) * 32767.).round() as i16)
            .collect();
        if let Some(encoder) = &mut encoder {
            for block in encoder.encode_interleaved(&pcm)? {
                writer.write_all(&block)?;
            }
        } else {
            for sample in pcm {
                writer.write_all(&sample.to_le_bytes())?;
            }
        }
        frame += count as u64;
    }
    if let Some(encoder) = &mut encoder {
        writer.write_all(&encoder.finish()?)?;
    }
    writer.flush()?;
    writer.get_ref().sync_all()?;
    Ok(())
}
