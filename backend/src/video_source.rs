use anyhow::{Result, ensure};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};
use symphonia::core::io::MediaSource;

pub type Patch = (u64, Vec<u8>);

struct Region {
    file: File,
    base: u64,
    length: u64,
    position: u64,
}

impl Read for Region {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        let length = bytes
            .len()
            .min(self.length.saturating_sub(self.position) as usize);
        let count = self.file.read(&mut bytes[..length])?;
        self.position += count as u64;
        Ok(count)
    }
}

impl Seek for Region {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        let position = match position {
            SeekFrom::Start(value) => value as i128,
            SeekFrom::Current(value) => self.position as i128 + value as i128,
            SeekFrom::End(value) => self.length as i128 + value as i128,
        };
        if position < 0 || position > self.length as i128 {
            return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput));
        }
        self.file
            .seek(SeekFrom::Start(self.base + position as u64))?;
        self.position = position as u64;
        Ok(self.position)
    }
}

impl MediaSource for Region {
    fn is_seekable(&self) -> bool {
        true
    }
    fn byte_len(&self) -> Option<u64> {
        Some(self.length)
    }
}

pub fn patches(path: &Path, base: u64, length: u64, name: &str) -> Result<Vec<Patch>> {
    let mut source = Region {
        file: File::open(path)?,
        base,
        length,
        position: 0,
    };
    source.seek(SeekFrom::Start(0))?;
    match Path::new(name)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_lowercase()
        .as_str()
    {
        "mp4" | "mov" | "m4v" => crate::mp4_timing::video_only_patches(&mut source),
        "mkv" | "webm" => matroska_patches(&mut source),
        _ => Ok(Vec::new()),
    }
}

struct Element {
    identifier: u64,
    start: u64,
    data: u64,
    end: u64,
}

fn integer(source: &mut dyn MediaSource, identifier: bool) -> Result<(u64, bool)> {
    let mut byte = [0];
    source.read_exact(&mut byte)?;
    let width = byte[0].leading_zeros() as usize + 1;
    ensure!(
        width <= if identifier { 4 } else { 8 },
        crate::i18n::message("error.unsupportedCodec")
    );
    let marker = 1u8 << (8 - width);
    let mut value = if identifier {
        byte[0]
    } else {
        byte[0] & (marker - 1)
    } as u64;
    for _ in 1..width {
        source.read_exact(&mut byte)?;
        value = (value << 8) | byte[0] as u64;
    }
    Ok((value, !identifier && value == (1u64 << (7 * width)) - 1))
}

fn elements(source: &mut dyn MediaSource, start: u64, end: u64, stop: u64) -> Result<Vec<Element>> {
    let mut position = start;
    let mut result = Vec::new();
    while position < end {
        source.seek(SeekFrom::Start(position))?;
        let (identifier, _) = integer(source, true)?;
        let (length, unknown) = integer(source, false)?;
        let data = source.stream_position()?;
        ensure!(
            data <= end && (unknown || length <= end - data) && result.len() < 4096,
            crate::i18n::message("error.unsupportedCodec")
        );
        let element_end = if unknown { end } else { data + length };
        result.push(Element {
            identifier,
            start: position,
            data,
            end: element_end,
        });
        position = element_end;
        if identifier == stop {
            break;
        }
    }
    Ok(result)
}

fn matroska_patches(source: &mut dyn MediaSource) -> Result<Vec<Patch>> {
    let length = source.byte_len().unwrap_or(0);
    let roots = elements(source, 0, length, 0x18538067)?;
    let Some(segment) = roots
        .iter()
        .find(|element| element.identifier == 0x18538067)
    else {
        return Ok(Vec::new());
    };
    let children = elements(source, segment.data, segment.end, 0x1654ae6b)?;
    let mut patches: Vec<_> = children
        .iter()
        .filter(|element| element.identifier == 0xbf)
        .map(|element| (element.start, vec![0xec]))
        .collect();
    let Some(tracks) = children
        .iter()
        .find(|element| element.identifier == 0x1654ae6b)
    else {
        return Ok(patches);
    };
    for track in elements(source, tracks.data, tracks.end, 0)? {
        if track.identifier == 0xbf {
            patches.push((track.start, vec![0xec]));
            continue;
        }
        if track.identifier != 0xae {
            continue;
        }
        let children = elements(source, track.data, track.end, 0x83)?;
        let Some(kind) = children.iter().find(|element| element.identifier == 0x83) else {
            continue;
        };
        ensure!(
            (1..=8).contains(&(kind.end - kind.data)),
            crate::i18n::message("error.unsupportedCodec")
        );
        source.seek(SeekFrom::Start(kind.data))?;
        let mut value = 0u64;
        for _ in kind.data..kind.end {
            let mut byte = [0];
            source.read_exact(&mut byte)?;
            value = (value << 8) | byte[0] as u64;
        }
        if value == 2 {
            patches.push((track.start, vec![0xec]));
        }
    }
    Ok(patches)
}

pub fn apply(patches: &[Patch], position: u64, bytes: &mut [u8]) {
    let end = position + bytes.len() as u64;
    for (start, replacement) in patches {
        let from = position.max(*start);
        let to = end.min(*start + replacement.len() as u64);
        if from < to {
            bytes[(from - position) as usize..(to - position) as usize]
                .copy_from_slice(&replacement[(from - start) as usize..(to - start) as usize]);
        }
    }
}
