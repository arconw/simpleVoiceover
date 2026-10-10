use crate::model::{Asset, MAX_FRAMES, SAMPLE_RATE};
use crate::{import_reader::ImportReader, progress::Progress};
use anyhow::{Context, Result, ensure};
use std::{
    fs::File,
    io::{BufWriter, SeekFrom, Write},
    path::{Path, PathBuf},
};
use symphonia::core::{
    audio::SampleBuffer,
    codecs::{CodecParameters, Decoder, DecoderOptions},
    errors::Error,
    formats::{FormatOptions, Packet},
    io::{MediaSource, MediaSourceStream},
    meta::MetadataOptions,
    probe::Hint,
    units::TimeBase,
};

#[cfg(test)]
pub fn decode(source: &Path, cache: &Path, asset: Asset) -> Result<Asset> {
    Ok(decode_stream(Box::new(File::open(source)?), cache, asset, None, false)?.remove(0))
}

pub fn import(
    source: &Path,
    owned: &Path,
    cache: &Path,
    asset: Asset,
    progress: &Progress,
) -> Result<Vec<Asset>> {
    let reader = ImportReader::new(
        source,
        owned,
        asset.size,
        crate::i18n::formatted_message("progress.import", &[("name", asset.name.clone())]),
        progress.clone(),
    )?;
    let decoded = decode_stream(Box::new(reader.clone()), cache, asset, Some(&reader), true)?;
    if let Err(error) = reader.finish() {
        for asset in &decoded {
            let _ = std::fs::remove_file(cache.with_file_name(format!("{}.pcm", asset.id)));
            if let Some(root) = owned.parent() {
                let _ = std::fs::remove_file(root.join(&asset.id));
            }
        }
        return Err(error);
    }
    Ok(decoded)
}

fn decode_stream(
    mut source: Box<dyn MediaSource>,
    cache: &Path,
    mut asset: Asset,
    import: Option<&ImportReader>,
    separate_streams: bool,
) -> Result<Vec<Asset>> {
    let mut hint = Hint::new();
    let mut timings = Ok(std::collections::HashMap::new());
    if let Some(extension) = Path::new(&asset.name).extension().and_then(|s| s.to_str()) {
        hint.with_extension(extension);
        if separate_streams && ["mp4", "mov", "m4v"].contains(&extension.to_lowercase().as_str()) {
            timings = crate::mp4_timing::read_timings(source.as_mut());
            source.seek(SeekFrom::Start(0))?;
        }
    }
    let stream = MediaSourceStream::new(source, Default::default());
    let probe = symphonia::default::get_probe().format(
        &hint,
        stream,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;
    let mut format = probe.format;
    let audio_tracks: Vec<_> = format
        .tracks()
        .iter()
        .filter(|track| track.codec_params.sample_rate.is_some())
        .cloned()
        .collect();
    let duration = format
        .tracks()
        .iter()
        .filter_map(|track| {
            let params = &track.codec_params;
            let time = params.time_base?.calc_time(params.n_frames?);
            Some(time.seconds as f64 + time.frac)
        })
        .fold(0., f64::max);
    let separate = separate_streams
        && asset.kind == "video"
        && (Path::new(&asset.name)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("mkv"))
            || audio_tracks.len() > 1
            || audio_tracks.is_empty());
    let timings = if separate {
        timings?
    } else {
        std::collections::HashMap::new()
    };
    ensure!(
        separate || !audio_tracks.is_empty(),
        crate::i18n::message("error.unsupportedCodec")
    );
    ensure!(
        audio_tracks.len() <= 126,
        crate::i18n::message("error.projectItemCount")
    );
    let mut created = Vec::new();
    let result = (|| {
        let mut streams = Vec::new();
        for (index, track) in audio_tracks.iter().enumerate() {
            if !separate && index > 0 {
                break;
            }
            let stream_asset = if separate {
                let stem: String = Path::new(&asset.name)
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .chars()
                    .take(100)
                    .collect();
                let language: String = track
                    .language
                    .as_deref()
                    .filter(|language| *language != "und")
                    .unwrap_or("")
                    .chars()
                    .take(16)
                    .collect();
                Asset {
                    id: crate::model::id(),
                    name: format!(
                        "{stem} [{}]{}.wav",
                        index + 1,
                        if language.is_empty() {
                            String::new()
                        } else {
                            format!(" {language}")
                        }
                    ),
                    kind: "audio".into(),
                    ..asset.clone()
                }
            } else {
                asset.clone()
            };
            let stream_cache = if separate {
                cache.with_file_name(format!("{}.pcm", stream_asset.id))
            } else {
                cache.to_path_buf()
            };
            created.push(stream_cache.clone());
            let mut decoder =
                AudioDecoder::new(track.id, &track.codec_params, stream_cache, stream_asset)?;
            if separate && let Some(timing) = timings.get(&track.id) {
                decoder.leading_frames = Some((timing.leading * SAMPLE_RATE as f64).round() as u64);
                decoder.skip_frames = (timing.trim * SAMPLE_RATE as f64).round() as u64;
            }
            streams.push(decoder);
        }
        if let Some(import) = import {
            import.expected_frames((duration * SAMPLE_RATE as f64) as u64);
        }
        loop {
            let packet = match format.next_packet() {
                Ok(p) => p,
                Err(Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e.into()),
            };
            if let Some(stream) = streams
                .iter_mut()
                .find(|stream| stream.track_id == packet.track_id())
            {
                stream.packet(&packet, separate)?;
                if let Some(import) = import {
                    import.decoded_frames(stream.peaks.frames);
                }
            }
        }
        let mut assets = Vec::new();
        for stream in streams {
            let stream_cache = stream.cache.clone();
            let mut decoded = stream.finish()?;
            if separate {
                let source = cache
                    .parent()
                    .context(crate::i18n::message("error.sourceCacheMissing"))?
                    .parent()
                    .context(crate::i18n::message("error.sourceCacheMissing"))?
                    .join("media")
                    .join(&decoded.id);
                created.push(source.clone());
                decoded.size = write_audio_source(&stream_cache, &source, decoded.frames)?;
            }
            assets.push(decoded);
        }
        if separate {
            asset.duration = assets
                .iter()
                .map(|audio| audio.duration)
                .fold(duration, f64::max);
            ensure!(
                asset.duration.is_finite()
                    && asset.duration > 0.
                    && asset.duration * SAMPLE_RATE as f64 <= MAX_FRAMES as f64,
                crate::i18n::message("error.mediaDuration")
            );
            asset.frames = 0;
            asset.waveform.clear();
            created.push(cache.to_path_buf());
            File::create(cache)?.sync_all()?;
            assets.insert(0, asset);
        }
        Ok(assets)
    })();
    if result.is_err() {
        for path in created {
            let _ = std::fs::remove_file(path);
        }
    }
    result
}

struct AudioDecoder {
    track_id: u32,
    decoder: Box<dyn Decoder>,
    time_base: Option<TimeBase>,
    output: BufWriter<File>,
    cache: PathBuf,
    asset: Asset,
    peaks: Peaks,
    resampler: Resampler,
    sample_buffer: Option<SampleBuffer<f32>>,
    pcm_bytes: Vec<u8>,
    input_rate: u32,
    leading_frames: Option<u64>,
    skip_frames: u64,
    timeline_started: bool,
}

impl AudioDecoder {
    fn new(
        track_id: u32,
        parameters: &CodecParameters,
        cache: PathBuf,
        asset: Asset,
    ) -> Result<Self> {
        let mut parameters = parameters.clone();
        parameters.max_frames_per_packet.get_or_insert(65_536);
        let decoder = symphonia::default::get_codecs()
            .make(&parameters, &DecoderOptions::default())
            .context(crate::i18n::message("error.unsupportedCodec"))?;
        Ok(Self {
            track_id,
            decoder,
            time_base: parameters.time_base,
            output: BufWriter::with_capacity(256 * 1024, File::create(&cache)?),
            cache,
            asset,
            peaks: Peaks::new(),
            resampler: Resampler::new(),
            sample_buffer: None,
            pcm_bytes: Vec::with_capacity(256 * 1024),
            input_rate: 0,
            leading_frames: None,
            skip_frames: 0,
            timeline_started: false,
        })
    }

    fn packet(&mut self, packet: &Packet, align: bool) -> Result<()> {
        let parameters = self.decoder.codec_params();
        if symphonia::default::get_codecs()
            .get_codec(parameters.codec)
            .is_some_and(|codec| codec.short_name.starts_with("pcm_"))
        {
            let channels = self.decoder.last_decoded().spec().channels.count().max(1);
            let maximum = packet.data.len() as u64 / channels as u64;
            if maximum > parameters.max_frames_per_packet.unwrap_or(0) {
                let mut parameters = parameters.clone();
                parameters.max_frames_per_packet = Some(maximum);
                self.decoder = symphonia::default::get_codecs()
                    .make(&parameters, &DecoderOptions::default())?;
            }
        }
        if align
            && !self.timeline_started
            && let Some(time_base) = self.time_base
        {
            let time = time_base.calc_time(packet.ts());
            let frames = self.leading_frames.unwrap_or_else(|| {
                ((time.seconds as f64 + time.frac) * SAMPLE_RATE as f64).round() as u64
            });
            self.timeline_started = true;
            ensure!(
                frames <= MAX_FRAMES,
                crate::i18n::message("error.mediaDuration")
            );
            let silence = [0; 8192];
            let mut remaining = frames * 8;
            while remaining > 0 {
                let count = remaining.min(silence.len() as u64) as usize;
                self.output.write_all(&silence[..count])?;
                for _ in 0..count / 8 {
                    self.peaks.push([0., 0.]);
                }
                remaining -= count as u64;
            }
        }
        let decoded = match self.decoder.decode(packet) {
            Ok(buffer) => buffer,
            Err(Error::DecodeError(_)) => return Ok(()),
            Err(e) => return Err(e.into()),
        };
        let rate = decoded.spec().rate;
        ensure!(
            (8000..=192000).contains(&rate) && (self.input_rate == 0 || self.input_rate == rate),
            crate::i18n::message("error.sampleRate")
        );
        self.input_rate = rate;
        let channels = decoded.spec().channels.count();
        ensure!(channels > 0, crate::i18n::message("error.noChannels"));
        let required = decoded.capacity() * channels;
        if self
            .sample_buffer
            .as_ref()
            .is_none_or(|buffer| buffer.capacity() < required)
        {
            self.sample_buffer = Some(SampleBuffer::<f32>::new(
                decoded.capacity() as u64,
                *decoded.spec(),
            ));
        }
        let samples = self.sample_buffer.as_mut().unwrap();
        samples.copy_interleaved_ref(decoded);
        self.pcm_bytes.clear();
        if rate == SAMPLE_RATE {
            for frame in samples.samples().chunks_exact(channels) {
                let sample = [
                    finite(frame[0]),
                    finite(if channels > 1 { frame[1] } else { frame[0] }),
                ];
                if self.skip_frames > 0 {
                    self.skip_frames -= 1;
                    continue;
                }
                self.pcm_bytes.extend_from_slice(&sample[0].to_le_bytes());
                self.pcm_bytes.extend_from_slice(&sample[1].to_le_bytes());
                self.peaks.push(sample);
            }
            ensure!(
                self.peaks.frames <= MAX_FRAMES,
                crate::i18n::message("error.mediaDuration")
            );
        } else {
            for frame in samples.samples().chunks_exact(channels) {
                let left = finite(frame[0]);
                let right = finite(if channels > 1 { frame[1] } else { frame[0] });
                self.resampler.push([left, right], rate, |sample| {
                    if self.skip_frames > 0 {
                        self.skip_frames -= 1;
                        return Ok(());
                    }
                    self.pcm_bytes.extend_from_slice(&sample[0].to_le_bytes());
                    self.pcm_bytes.extend_from_slice(&sample[1].to_le_bytes());
                    self.peaks.push(sample);
                    ensure!(
                        self.peaks.frames <= MAX_FRAMES,
                        crate::i18n::message("error.mediaDuration")
                    );
                    Ok(())
                })?;
            }
        }
        self.output.write_all(&self.pcm_bytes)?;
        Ok(())
    }

    fn finish(mut self) -> Result<Asset> {
        self.resampler.finish(|sample| {
            if self.skip_frames > 0 {
                self.skip_frames -= 1;
                return Ok(());
            }
            self.output.write_all(&sample[0].to_le_bytes())?;
            self.output.write_all(&sample[1].to_le_bytes())?;
            self.peaks.push(sample);
            Ok(())
        })?;
        self.output.flush()?;
        self.output.get_ref().sync_all()?;
        ensure!(
            self.peaks.frames > 0 && self.peaks.frames <= MAX_FRAMES,
            crate::i18n::message("error.noDecodableAudio")
        );
        self.asset.frames = self.peaks.frames;
        self.asset.duration = self.peaks.frames as f64 / SAMPLE_RATE as f64;
        self.asset.sample_rate = SAMPLE_RATE;
        self.peaks.finish();
        self.asset.waveform = self.peaks.bins;
        self.asset.peak_frames = self.peaks.width;
        Ok(self.asset)
    }
}

fn write_audio_source(cache: &Path, source: &Path, frames: u64) -> Result<u64> {
    let bytes = frames * 8;
    let mut header = Vec::new();
    if bytes + 36 > u32::MAX as u64 {
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
    header.extend(3u16.to_le_bytes());
    header.extend(2u16.to_le_bytes());
    header.extend(SAMPLE_RATE.to_le_bytes());
    header.extend((SAMPLE_RATE * 8).to_le_bytes());
    header.extend(8u16.to_le_bytes());
    header.extend(32u16.to_le_bytes());
    header.extend(b"data");
    header.extend(
        if bytes > u32::MAX as u64 {
            u32::MAX
        } else {
            bytes as u32
        }
        .to_le_bytes(),
    );
    let mut output = BufWriter::new(File::create(source)?);
    output.write_all(&header)?;
    std::io::copy(&mut File::open(cache)?, &mut output)?;
    output.flush()?;
    output.get_ref().sync_all()?;
    Ok(header.len() as u64 + bytes)
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
            crate::i18n::message("error.sampleRate")
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
