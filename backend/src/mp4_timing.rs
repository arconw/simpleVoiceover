use anyhow::{Result, ensure};
use std::{collections::HashMap, io::SeekFrom};
use symphonia::core::io::MediaSource;

#[derive(Clone, Copy, Default)]
pub struct Timing {
    pub leading: f64,
    pub trim: f64,
}

struct Atom {
    kind: [u8; 4],
    start: u64,
    data: u64,
    end: u64,
}

fn read<const N: usize>(source: &mut dyn MediaSource, position: u64) -> Result<[u8; N]> {
    source.seek(SeekFrom::Start(position))?;
    let mut bytes = [0; N];
    source.read_exact(&mut bytes)?;
    Ok(bytes)
}

fn children(
    source: &mut dyn MediaSource,
    start: u64,
    end: u64,
    stop_after: Option<[u8; 4]>,
) -> Result<Vec<Atom>> {
    let mut position = start;
    let mut result = Vec::new();
    while end.saturating_sub(position) >= 8 {
        let header = read::<8>(source, position)?;
        let length = u32::from_be_bytes(header[..4].try_into().unwrap()) as u64;
        let (length, width) = match length {
            0 => (end - position, 8),
            1 => (u64::from_be_bytes(read(source, position + 8)?), 16),
            length => (length, 8),
        };
        ensure!(
            length >= width && length <= end - position && result.len() < 4096,
            crate::i18n::message("error.unsupportedCodec")
        );
        result.push(Atom {
            kind: header[4..].try_into().unwrap(),
            start: position,
            data: position + width,
            end: position + length,
        });
        if stop_after == result.last().map(|atom| atom.kind) {
            break;
        }
        position += length;
    }
    Ok(result)
}

fn timescale(source: &mut dyn MediaSource, atom: &Atom) -> Result<u32> {
    let version = read::<1>(source, atom.data)?[0];
    ensure!(version <= 1, crate::i18n::message("error.unsupportedCodec"));
    let offset = if version == 1 { 20 } else { 12 };
    ensure!(
        atom.end.saturating_sub(atom.data) >= offset + 4,
        crate::i18n::message("error.unsupportedCodec")
    );
    let value = u32::from_be_bytes(read(source, atom.data + offset)?);
    ensure!(value > 0, crate::i18n::message("error.unsupportedCodec"));
    Ok(value)
}

pub fn read_timings(source: &mut dyn MediaSource) -> Result<HashMap<u32, Timing>> {
    let mut result = HashMap::new();
    let length = source.byte_len().unwrap_or(0);
    let atoms = children(source, 0, length, Some(*b"moov"))?;
    let Some(movie) = atoms.iter().find(|atom| atom.kind == *b"moov") else {
        return Ok(result);
    };
    let atoms = children(source, movie.data, movie.end, None)?;
    let Some(header) = atoms.iter().find(|atom| atom.kind == *b"mvhd") else {
        return Ok(result);
    };
    let movie_scale = timescale(source, header)? as f64;
    for (index, track) in atoms
        .iter()
        .filter(|atom| atom.kind == *b"trak")
        .enumerate()
    {
        let atoms = children(source, track.data, track.end, None)?;
        let Some(edits) = atoms.iter().find(|atom| atom.kind == *b"edts") else {
            continue;
        };
        let Some(media) = atoms.iter().find(|atom| atom.kind == *b"mdia") else {
            continue;
        };
        let media_atoms = children(source, media.data, media.end, None)?;
        if let Some(handler) = media_atoms.iter().find(|atom| atom.kind == *b"hdlr") {
            ensure!(
                handler.end.saturating_sub(handler.data) >= 12,
                crate::i18n::message("error.unsupportedCodec")
            );
            if read::<4>(source, handler.data + 8)? != *b"soun" {
                continue;
            }
        }
        let Some(header) = media_atoms.iter().find(|atom| atom.kind == *b"mdhd") else {
            continue;
        };
        let media_scale = timescale(source, header)? as f64;
        let edits = children(source, edits.data, edits.end, None)?;
        let Some(edits) = edits.iter().find(|atom| atom.kind == *b"elst") else {
            continue;
        };
        let header = read::<8>(source, edits.data)?;
        let version = header[0];
        let count = u32::from_be_bytes(header[4..].try_into().unwrap());
        let width = if version == 1 { 20 } else { 12 };
        ensure!(
            version <= 1
                && count <= 128
                && edits.end.saturating_sub(edits.data) >= 8 + count as u64 * width,
            crate::i18n::message("error.unsupportedCodec")
        );
        let mut timing = Timing::default();
        for entry in 0..count {
            let position = edits.data + 8 + entry as u64 * width;
            let (duration, time, rate_offset) = if version == 1 {
                (
                    u64::from_be_bytes(read(source, position)?),
                    i64::from_be_bytes(read(source, position + 8)?),
                    16,
                )
            } else {
                (
                    u32::from_be_bytes(read(source, position)?) as u64,
                    i32::from_be_bytes(read(source, position + 4)?) as i64,
                    8,
                )
            };
            ensure!(
                read::<4>(source, position + rate_offset)? == [0, 1, 0, 0],
                crate::i18n::message("error.unsupportedCodec")
            );
            if time == -1 {
                timing.leading += duration as f64 / movie_scale;
            } else {
                ensure!(
                    time >= 0 && entry + 1 == count,
                    crate::i18n::message("error.unsupportedCodec")
                );
                timing.trim = time as f64 / media_scale;
                break;
            }
        }
        result.insert(index as u32, timing);
    }
    Ok(result)
}

pub fn video_only_patches(source: &mut dyn MediaSource) -> Result<Vec<(u64, Vec<u8>)>> {
    let mut patches = Vec::new();
    let length = source.byte_len().unwrap_or(0);
    let atoms = children(source, 0, length, Some(*b"moov"))?;
    let Some(movie) = atoms.iter().find(|atom| atom.kind == *b"moov") else {
        return Ok(patches);
    };
    for track in children(source, movie.data, movie.end, None)?
        .into_iter()
        .filter(|atom| atom.kind == *b"trak")
    {
        let atoms = children(source, track.data, track.end, None)?;
        let Some(media) = atoms.iter().find(|atom| atom.kind == *b"mdia") else {
            continue;
        };
        let atoms = children(source, media.data, media.end, None)?;
        let Some(handler) = atoms.iter().find(|atom| atom.kind == *b"hdlr") else {
            continue;
        };
        ensure!(
            handler.end - handler.data >= 12,
            crate::i18n::message("error.unsupportedCodec")
        );
        if read::<4>(source, handler.data + 8)? == *b"soun" {
            patches.push((track.start + 4, b"free".to_vec()));
        }
    }
    Ok(patches)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs::File,
        io::{Cursor, Seek, Write},
    };

    fn atom(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut bytes = ((data.len() + 8) as u32).to_be_bytes().to_vec();
        bytes.extend(kind);
        bytes.extend(data);
        bytes
    }

    fn movie(version: u8) -> Vec<u8> {
        let mut header = vec![0; if version == 1 { 20 } else { 12 }];
        header[0] = version;
        header.extend(1000u32.to_be_bytes());
        let mut media = vec![0; if version == 1 { 20 } else { 12 }];
        media[0] = version;
        media.extend(48000u32.to_be_bytes());
        let mut edits = vec![version, 0, 0, 0];
        edits.extend(2u32.to_be_bytes());
        for (duration, start) in [(500, -1), (2000, 1024)] {
            if version == 1 {
                edits.extend((duration as u64).to_be_bytes());
                edits.extend((start as i64).to_be_bytes());
            } else {
                edits.extend((duration as u32).to_be_bytes());
                edits.extend((start as i32).to_be_bytes());
            }
            edits.extend([0, 1, 0, 0]);
        }
        let mut track = atom(b"mdia", &atom(b"mdhd", &media));
        track.extend(atom(b"edts", &atom(b"elst", &edits)));
        let mut movie = atom(b"mvhd", &header);
        movie.extend(atom(b"trak", &track));
        atom(b"moov", &movie)
    }

    #[test]
    fn edit_lists_keep_leading_silence_and_encoder_trim_for_both_versions() -> Result<()> {
        for version in [0, 1] {
            let timings = read_timings(&mut Cursor::new(movie(version)))?;
            assert_eq!(timings[&0].leading, 0.5);
            assert!((timings[&0].trim - 1024. / 48000.).abs() < 1e-10);
        }
        Ok(())
    }

    #[test]
    fn metadata_after_large_media_is_read_without_loading_the_media() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let mut file = File::create(directory.path().join("large.mp4"))?;
        let media_size = (1u64 << 32) + 128;
        file.write_all(&1u32.to_be_bytes())?;
        file.write_all(b"mdat")?;
        file.write_all(&media_size.to_be_bytes())?;
        file.seek(SeekFrom::Start(media_size))?;
        file.write_all(&movie(1))?;
        drop(file);
        let timings = read_timings(&mut File::open(directory.path().join("large.mp4"))?)?;
        assert_eq!(timings[&0].leading, 0.5);
        Ok(())
    }

    #[test]
    fn invalid_atom_lengths_are_rejected() {
        for bytes in [
            vec![0, 0, 0, 2, b'm', b'o', b'o', b'v'],
            vec![0, 0, 0, 99, b'm', b'o', b'o', b'v'],
        ] {
            assert!(read_timings(&mut Cursor::new(bytes)).is_err());
        }
    }
}
