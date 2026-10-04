use crate::model::{Asset, MAX_FRAMES, SAMPLE_RATE};
use anyhow::{Context, Result, ensure};
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};
use symphonia::core::{
    audio::SampleBuffer, codecs::DecoderOptions, errors::Error, formats::FormatOptions,
    io::MediaSourceStream, meta::MetadataOptions, probe::Hint,
};

pub fn decode(source: &Path, cache: &Path, mut asset: Asset) -> Result<Asset> {
    let mut hint = Hint::new();
    if let Some(extension) = Path::new(&asset.name).extension().and_then(|s| s.to_str()) {
        hint.with_extension(extension);
    }
    let stream = MediaSourceStream::new(Box::new(File::open(source)?), Default::default());
    let probe = symphonia::default::get_probe().format(
        &hint,
        stream,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;
    let mut format = probe.format;
    let track = format.tracks().iter().find(|t| t.codec_params.sample_rate.is_some() && symphonia::default::get_codecs().make(&t.codec_params, &DecoderOptions::default()).is_ok()).context("Аудиокодек не поддерживается Rust-движком. Используй AAC-LC, MP3, WAV, FLAC, ALAC или Vorbis.")?;
    let track_id = track.id;
    let mut decoder =
        symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default())?;
    let mut output = BufWriter::with_capacity(256 * 1024, File::create(cache)?);
    let mut peaks = Peaks::new();
    let mut resampler = Resampler::new();
    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e.into()),
        };
        if packet.track_id() != track_id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(buffer) => buffer,
            Err(Error::DecodeError(_)) => continue,
            Err(e) => return Err(e.into()),
        };
        let rate = decoded.spec().rate;
        let channels = decoded.spec().channels.count();
        ensure!(channels > 0, "Нет аудиоканалов");
        let mut samples = SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
        samples.copy_interleaved_ref(decoded);
        for frame in samples.samples().chunks_exact(channels) {
            let left = finite(frame[0]);
            let right = finite(if channels > 1 { frame[1] } else { frame[0] });
            resampler.push([left, right], rate, |sample| {
                output.write_all(&sample[0].to_le_bytes())?;
                output.write_all(&sample[1].to_le_bytes())?;
                peaks.push(sample);
                ensure!(
                    peaks.frames <= MAX_FRAMES,
                    "Медиа превышает предельную длительность проекта"
                );
                Ok(())
            })?;
        }
    }
    resampler.finish(|sample| {
        output.write_all(&sample[0].to_le_bytes())?;
        output.write_all(&sample[1].to_le_bytes())?;
        peaks.push(sample);
        Ok(())
    })?;
    output.flush()?;
    output.get_ref().sync_all()?;
    ensure!(peaks.frames > 0, "В файле нет декодируемого звука");
    asset.frames = peaks.frames;
    asset.duration = peaks.frames as f64 / SAMPLE_RATE as f64;
    asset.sample_rate = SAMPLE_RATE;
    peaks.finish();
    asset.waveform = peaks.bins;
    asset.peak_frames = peaks.width;
    Ok(asset)
}

pub fn finite(sample: f32) -> f32 {
    if sample.is_finite() {
        sample.clamp(-8., 8.)
    } else {
        0.
    }
}

pub struct Resampler {
    previous: Option<[f32; 2]>,
    input_index: u64,
    next_output: f64,
    rate: u32,
}
impl Resampler {
    pub fn new() -> Self {
        Self {
            previous: None,
            input_index: 0,
            next_output: 0.,
            rate: 0,
        }
    }
    pub fn push(
        &mut self,
        sample: [f32; 2],
        rate: u32,
        mut write: impl FnMut([f32; 2]) -> Result<()>,
    ) -> Result<()> {
        ensure!(
            (8000..=192000).contains(&rate) && (self.rate == 0 || self.rate == rate),
            "Неподдерживаемая частота аудио"
        );
        self.rate = rate;
        if let Some(previous) = self.previous {
            while self.next_output <= self.input_index as f64 {
                let fraction =
                    (self.next_output - (self.input_index - 1) as f64).clamp(0., 1.) as f32;
                write([
                    previous[0] + (sample[0] - previous[0]) * fraction,
                    previous[1] + (sample[1] - previous[1]) * fraction,
                ])?;
                self.next_output += rate as f64 / SAMPLE_RATE as f64;
            }
        }
        self.previous = Some(sample);
        self.input_index += 1;
        Ok(())
    }
    pub fn finish(&mut self, mut write: impl FnMut([f32; 2]) -> Result<()>) -> Result<()> {
        if let Some(sample) = self.previous {
            while self.next_output < self.input_index as f64 {
                write(sample)?;
                self.next_output += self.rate as f64 / SAMPLE_RATE as f64;
            }
        }
        Ok(())
    }
}

pub struct Peaks {
    pub bins: Vec<[f32; 2]>,
    pub width: u64,
    pub frames: u64,
    current: [f32; 2],
    count: u64,
}
impl Peaks {
    pub fn new() -> Self {
        Self {
            bins: vec![],
            width: 256,
            frames: 0,
            current: [0., 0.],
            count: 0,
        }
    }
    pub fn push(&mut self, sample: [f32; 2]) {
        self.current[0] = self.current[0].min(sample[0]).min(sample[1]);
        self.current[1] = self.current[1].max(sample[0]).max(sample[1]);
        self.frames += 1;
        self.count += 1;
        if self.count == self.width {
            self.bins.push(self.current);
            self.current = [0., 0.];
            self.count = 0;
        }
        if self.bins.len() == 65536 {
            self.bins = self
                .bins
                .chunks_exact(2)
                .map(|p| [p[0][0].min(p[1][0]), p[0][1].max(p[1][1])])
                .collect();
            self.width *= 2;
        }
    }
    pub fn finish(&mut self) {
        if self.count > 0 {
            self.bins.push(self.current);
            self.count = 0;
        }
    }
}

pub fn wav_header(frames: u64) -> Vec<u8> {
    let bytes = frames * 4;
    let large = bytes + 36 > u32::MAX as u64;
    let mut header = vec![];
    if large {
        header.extend(b"RF64");
        header.extend(u32::MAX.to_le_bytes());
        header.extend(b"WAVEds64");
        header.extend(28u32.to_le_bytes());
        header.extend((bytes + 72).to_le_bytes());
        header.extend(bytes.to_le_bytes());
        header.extend(frames.to_le_bytes());
        header.extend(0u32.to_le_bytes());
    } else {
        header.extend(b"RIFF");
        header.extend((bytes as u32 + 36).to_le_bytes());
        header.extend(b"WAVE");
    }
    header.extend(b"fmt ");
    header.extend(16u32.to_le_bytes());
    header.extend(1u16.to_le_bytes());
    header.extend(2u16.to_le_bytes());
    header.extend(SAMPLE_RATE.to_le_bytes());
    header.extend((SAMPLE_RATE * 4).to_le_bytes());
    header.extend(4u16.to_le_bytes());
    header.extend(16u16.to_le_bytes());
    header.extend(b"data");
    header.extend(if large { u32::MAX } else { bytes as u32 }.to_le_bytes());
    header
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resampling_and_rf64_keep_long_audio_bounded() -> Result<()> {
        let mut resampler = Resampler::new();
        let mut count = 0;
        for _ in 0..44100 {
            resampler.push([0.5, -0.5], 44100, |x| {
                assert_eq!(x, [0.5, -0.5]);
                count += 1;
                Ok(())
            })?;
        }
        resampler.finish(|_| {
            count += 1;
            Ok(())
        })?;
        assert!((count as i64 - 48000).abs() <= 1);
        assert_eq!(&wav_header(2_000_000_000)[..4], b"RF64");
        assert_eq!(wav_header(48000).len(), 44);
        Ok(())
    }
}
