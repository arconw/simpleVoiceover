use crate::progress::Progress;
use anyhow::{Result, ensure};
use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
use symphonia::core::io::MediaSource;

struct CopyState {
    source: File,
    owned: File,
    position: u64,
    copied: u64,
    length: u64,
    ranges: BTreeMap<u64, u64>,
}

impl CopyState {
    fn copied_end(&self, position: u64) -> Option<u64> {
        self.ranges
            .range(..=position)
            .next_back()
            .map(|(_, end)| *end)
            .filter(|end| *end > position)
    }

    fn record_range(&mut self, mut start: u64, mut end: u64) {
        self.copied += end - start;
        if let Some((&previous_start, &previous_end)) = self.ranges.range(..start).next_back()
            && previous_end == start
        {
            self.ranges.remove(&previous_start);
            start = previous_start;
        }
        if let Some(next_end) = self.ranges.remove(&end) {
            end = next_end;
        }
        self.ranges.insert(start, end);
    }
}

struct ImportProgress {
    reporter: Progress,
    label: String,
    copied: AtomicU64,
    length: u64,
    decoded: AtomicU64,
    expected: AtomicU64,
}

impl ImportProgress {
    fn report(&self) {
        let copy = self.copied.load(Ordering::Relaxed) as f64 / self.length.max(1) as f64;
        let expected = self.expected.load(Ordering::Relaxed);
        let decode = if expected > 0 {
            (self.decoded.load(Ordering::Relaxed) as f64 / expected as f64).min(1.)
        } else {
            0.
        };
        self.reporter
            .report(&self.label, (copy * 0.65 + decode * 0.35) * 0.99);
    }
}

#[derive(Clone)]
pub struct ImportReader {
    state: Arc<Mutex<CopyState>>,
    progress: Arc<ImportProgress>,
}

impl ImportReader {
    pub fn new(
        source: &Path,
        owned: &Path,
        expected_size: u64,
        label: String,
        reporter: Progress,
    ) -> Result<Self> {
        let source = File::open(source)?;
        ensure!(
            source.metadata()?.len() == expected_size,
            crate::i18n::message("error.sourceSizeChanged")
        );
        let owned = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(owned)?;
        let reader = Self {
            state: Arc::new(Mutex::new(CopyState {
                source,
                owned,
                position: 0,
                copied: 0,
                length: expected_size,
                ranges: BTreeMap::new(),
            })),
            progress: Arc::new(ImportProgress {
                reporter,
                label,
                copied: AtomicU64::new(0),
                length: expected_size,
                decoded: AtomicU64::new(0),
                expected: AtomicU64::new(0),
            }),
        };
        reader.progress.report();
        Ok(reader)
    }

    pub fn expected_frames(&self, frames: u64) {
        self.progress.expected.store(frames, Ordering::Relaxed);
    }

    pub fn decoded_frames(&self, frames: u64) {
        self.progress.decoded.store(frames, Ordering::Relaxed);
        self.progress.report();
    }

    fn copy_remaining(&self) -> io::Result<()> {
        let mut reader = self.clone();
        let mut buffer = vec![0; 1024 * 1024];
        let mut position = 0;
        while position < self.progress.length {
            let end = self.state.lock().unwrap().copied_end(position);
            if let Some(end) = end {
                position = end;
                continue;
            }
            reader.seek(SeekFrom::Start(position))?;
            let count = reader.read(&mut buffer)?;
            if count == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    crate::i18n::message("error.sourceTruncated"),
                ));
            }
            position += count as u64;
        }
        Ok(())
    }

    pub fn finish(&self) -> Result<()> {
        self.progress.expected.store(1, Ordering::Relaxed);
        self.progress.decoded.store(1, Ordering::Relaxed);
        self.copy_remaining()?;
        let state = self.state.lock().unwrap();
        let length = state.length;
        ensure!(
            state.source.metadata()?.len() == length,
            crate::i18n::message("error.sourceChanged")
        );
        state.owned.sync_all()?;
        self.progress.reporter.report(&self.progress.label, 1.);
        Ok(())
    }
}

impl Read for ImportReader {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        let mut state = self.state.lock().unwrap();
        if bytes.is_empty() || state.position >= state.length {
            return Ok(0);
        }
        let position = state.position;
        let copied_end = state.copied_end(position);
        let limit = copied_end.unwrap_or_else(|| {
            state
                .ranges
                .range(position..)
                .next()
                .map(|(start, _)| *start)
                .unwrap_or(state.length)
        });
        let count = bytes.len().min((limit - position) as usize);
        let count = if copied_end.is_some() {
            state.owned.seek(SeekFrom::Start(position))?;
            state.owned.read(&mut bytes[..count])?
        } else {
            state.source.seek(SeekFrom::Start(position))?;
            let read = state.source.read(&mut bytes[..count])?;
            if read == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    crate::i18n::message("error.sourceTruncated"),
                ));
            }
            state.owned.seek(SeekFrom::Start(position))?;
            state.owned.write_all(&bytes[..read])?;
            state.record_range(position, position + read as u64);
            self.progress.copied.store(state.copied, Ordering::Relaxed);
            read
        };
        state.position += count as u64;
        self.progress.report();
        Ok(count)
    }
}

impl Seek for ImportReader {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        let mut state = self.state.lock().unwrap();
        let position = match from {
            SeekFrom::Start(value) => value,
            SeekFrom::Current(delta) => {
                state.position.checked_add_signed(delta).ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        crate::i18n::message("error.position"),
                    )
                })?
            }
            SeekFrom::End(delta) => state.length.checked_add_signed(delta).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    crate::i18n::message("error.position"),
                )
            })?,
        };
        state.position = position;
        Ok(position)
    }
}

impl MediaSource for ImportReader {
    fn is_seekable(&self) -> bool {
        true
    }
    fn byte_len(&self) -> Option<u64> {
        Some(self.progress.length)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn seeks_copy_gaps_and_finish_keeps_unread_video_bytes() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let source = temp.path().join("source.bin");
        let owned = temp.path().join("owned.bin");
        let bytes: Vec<_> = (0..2_000_000).map(|index| (index % 251) as u8).collect();
        fs::write(&source, &bytes)?;
        let mut reader = ImportReader::new(
            &source,
            &owned,
            bytes.len() as u64,
            "Importing".into(),
            Progress::default(),
        )?;
        let mut block = [0; 333];
        reader.read_exact(&mut block)?;
        assert_eq!(&block, &bytes[..333]);
        reader.seek(SeekFrom::End(-500))?;
        reader.read_exact(&mut block)?;
        assert_eq!(&block, &bytes[bytes.len() - 500..bytes.len() - 167]);
        assert_eq!(reader.state.lock().unwrap().copied, 666);
        reader.seek(SeekFrom::Start(20))?;
        reader.read_exact(&mut block)?;
        assert_eq!(&block, &bytes[20..353]);
        reader.seek(SeekFrom::Start(bytes.len() as u64 - 167))?;
        reader.read_exact(&mut block[..167])?;
        assert_eq!(&block[..167], &bytes[bytes.len() - 167..]);
        assert_eq!(reader.read(&mut block)?, 0);
        reader.seek(SeekFrom::Start(bytes.len() as u64 + 100))?;
        assert_eq!(reader.read(&mut block)?, 0);
        assert!(reader.seek(SeekFrom::End(-3_000_000)).is_err());
        reader.finish()?;
        assert_eq!(reader.state.lock().unwrap().ranges.len(), 1);
        assert_eq!(reader.state.lock().unwrap().copied, bytes.len() as u64);
        assert_eq!(fs::read(&owned)?, bytes);
        let other = temp.path().join("partial.bin");
        let mut partial = ImportReader::new(
            &source,
            &other,
            bytes.len() as u64,
            "Importing".into(),
            Progress::default(),
        )?;
        partial.read_exact(&mut block)?;
        partial.finish()?;
        assert_eq!(fs::read(&other)?, bytes);
        Ok(())
    }
}
