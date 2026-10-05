use crate::{
    archive::{Archive, MAX_MANIFEST, Region, Revision, asset_entries},
    filesystem::replace_file,
    model::Project,
    progress::Progress,
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};

pub const MAGIC: [u8; 8] = *b"SVOPRJ3\0";
const SLOT_SIZE: usize = 40;
const HEADER_SIZE: u64 = 8 + 2 * SLOT_SIZE as u64;

#[derive(Serialize, Deserialize)]
struct Document {
    project: Project,
    entries: BTreeMap<String, Region>,
}

impl Revision {
    fn encode(&self) -> [u8; SLOT_SIZE] {
        let mut bytes = [0; SLOT_SIZE];
        for (index, value) in [
            self.generation,
            self.offset,
            self.length,
            self.committed_length,
        ]
        .into_iter()
        .enumerate()
        {
            bytes[index * 8..index * 8 + 8].copy_from_slice(&value.to_le_bytes());
        }
        bytes[32..36].copy_from_slice(&self.checksum.to_le_bytes());
        let checksum = crc32fast::hash(&bytes[..36]);
        bytes[36..].copy_from_slice(&checksum.to_le_bytes());
        bytes
    }

    fn decode(bytes: &[u8]) -> Option<Self> {
        let checksum = u32::from_le_bytes(bytes[36..40].try_into().ok()?);
        if crc32fast::hash(&bytes[..36]) != checksum {
            return None;
        }
        let number = |index| u64::from_le_bytes(bytes[index..index + 8].try_into().unwrap());
        Some(Self {
            generation: number(0),
            offset: number(8),
            length: number(16),
            committed_length: number(24),
            checksum: u32::from_le_bytes(bytes[32..36].try_into().ok()?),
        })
    }
}

pub fn read(mut file: File, path: &Path) -> Result<(Project, Archive)> {
    let file_length = file.metadata()?.len();
    let mut header = [0; HEADER_SIZE as usize];
    file.read_exact(&mut header)?;
    let mut revisions: Vec<_> = header[8..]
        .chunks_exact(SLOT_SIZE)
        .filter_map(Revision::decode)
        .collect();
    revisions.sort_by_key(|revision| std::cmp::Reverse(revision.generation));
    for revision in revisions {
        if let Ok(document) = read_document(&mut file, file_length, &revision) {
            return Ok((
                document.project,
                Archive {
                    path: path.to_path_buf(),
                    entries: document.entries,
                    revision: Some(revision),
                },
            ));
        }
    }
    anyhow::bail!(crate::i18n::message("error.noValidRevision"))
}

fn read_document(file: &mut File, file_length: u64, revision: &Revision) -> Result<Document> {
    ensure!(
        revision.generation > 0
            && revision.offset >= HEADER_SIZE
            && revision.length > 0
            && revision.length <= MAX_MANIFEST
            && revision.offset.checked_add(revision.length) == Some(revision.committed_length)
            && revision.committed_length <= file_length,
        crate::i18n::message("error.corruptIndex")
    );
    file.seek(SeekFrom::Start(revision.offset))?;
    let mut bytes = vec![0; revision.length as usize];
    file.read_exact(&mut bytes)?;
    ensure!(
        crc32fast::hash(&bytes) == revision.checksum,
        crate::i18n::message("error.corruptManifest")
    );
    let document: Document = serde_json::from_slice(&bytes)?;
    document.project.validate()?;
    ensure!(
        document.project.version == 3,
        crate::i18n::message("error.projectVersion")
    );
    ensure!(
        document.entries.len() == document.project.assets.len() * 2,
        crate::i18n::message("error.mediaIndex")
    );
    let mut regions = Vec::with_capacity(document.entries.len());
    for (name, expected) in asset_entries(&document.project) {
        let region = document
            .entries
            .get(&name)
            .context(crate::i18n::message("error.mediaMissingFromIndex"))?;
        let end = region
            .offset
            .checked_add(region.length)
            .context(crate::i18n::message("error.offset"))?;
        ensure!(
            region.length == expected && region.offset >= HEADER_SIZE && end <= revision.offset,
            crate::i18n::message("error.corruptMedia")
        );
        if region.length > 0 {
            regions.push((region.offset, end));
        }
    }
    regions.sort_unstable();
    ensure!(
        regions.windows(2).all(|pair| pair[0].1 <= pair[1].0),
        crate::i18n::message("error.overlappingMedia")
    );
    Ok(document)
}

pub fn save(
    project: &Project,
    root: &Path,
    source: Option<&Archive>,
    destination: &Path,
    compact: bool,
    progress: &Progress,
) -> Result<Archive> {
    project.validate()?;
    let same_file = source.is_some_and(|archive| {
        archive.path == destination
            || fs::canonicalize(&archive.path)
                .ok()
                .zip(fs::canonicalize(destination).ok())
                .is_some_and(|(left, right)| left == right)
    });
    if let Some(source) =
        source.filter(|archive| archive.revision.is_some() && (!same_file || compact))
    {
        let (_, current) = Archive::open(&source.path)?;
        ensure!(
            current.revision == source.revision,
            crate::i18n::message("error.projectChanged")
        );
    }
    if !compact && same_file && source.unwrap().revision.is_some() {
        let source = source.unwrap();
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(destination)?;
        file.try_lock()
            .context(crate::i18n::message("error.projectSavingElsewhere"))?;
        let (_, current) = Archive::open(destination)?;
        ensure!(
            current.revision == source.revision,
            crate::i18n::message("error.projectChanged")
        );
        let previous = source.revision.as_ref().unwrap();
        file.set_len(previous.committed_length)?;
        file.seek(SeekFrom::Start(previous.committed_length))?;
        return write_revision(
            &mut file,
            project,
            root,
            Some(source),
            true,
            destination,
            progress,
        );
    }
    let temporary = destination.with_extension(format!("{}.partial", crate::model::id()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(&MAGIC)?;
        file.write_all(&[0; 2 * SLOT_SIZE])?;
        let archive = write_revision(
            &mut file,
            project,
            root,
            source,
            false,
            destination,
            progress,
        )?;
        drop(file);
        replace_file(&temporary, destination)?;
        Ok(archive)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn write_revision(
    file: &mut File,
    project: &Project,
    root: &Path,
    source: Option<&Archive>,
    reuse: bool,
    destination: &Path,
    progress: &Progress,
) -> Result<Archive> {
    let generation = if reuse {
        source
            .unwrap()
            .revision
            .as_ref()
            .unwrap()
            .generation
            .checked_add(1)
            .context(crate::i18n::message("error.revisionOverflow"))?
    } else {
        1
    };
    let mut entries = BTreeMap::new();
    let total: u64 = asset_entries(project)
        .filter(|(name, _)| !reuse || !source.unwrap().entries.contains_key(name))
        .map(|(_, size)| size)
        .sum();
    let mut copied = 0;
    let mut buffer = vec![0; 1024 * 1024];
    progress.report(&crate::i18n::message("progress.saveMedia"), 0.);
    for (name, expected) in asset_entries(project) {
        if reuse && let Some(region) = source.unwrap().entries.get(&name) {
            ensure!(
                region.length == expected,
                crate::i18n::message("error.savedMediaSizeChanged")
            );
            entries.insert(name, region.clone());
            continue;
        }
        let (path, base, length) =
            if let Some(region) = source.and_then(|archive| archive.entries.get(&name)) {
                (source.unwrap().path.clone(), region.offset, region.length)
            } else {
                let path = root.join(&name);
                let length = fs::metadata(&path)?.len();
                (path, 0, length)
            };
        ensure!(length == expected, crate::i18n::message("error.mediaSize"));
        let mut input = File::open(path)?;
        input.seek(SeekFrom::Start(base))?;
        let offset = file.stream_position()?;
        let mut remaining = length;
        while remaining > 0 {
            let count = remaining.min(buffer.len() as u64) as usize;
            input.read_exact(&mut buffer[..count])?;
            file.write_all(&buffer[..count])?;
            remaining -= count as u64;
            copied += count as u64;
            progress.report(
                &crate::i18n::message("progress.saveMedia"),
                copied as f64 / total.max(1) as f64 * 0.95,
            );
        }
        entries.insert(name, Region { offset, length });
    }
    progress.report(&crate::i18n::message("progress.saveIndex"), 0.96);
    let mut saved_project = project.clone();
    saved_project.version = 3;
    let document = Document {
        project: saved_project,
        entries,
    };
    let bytes = serde_json::to_vec(&document)?;
    ensure!(
        bytes.len() as u64 <= MAX_MANIFEST,
        crate::i18n::message("error.manifestSize")
    );
    let offset = file.stream_position()?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    let revision = Revision {
        generation,
        offset,
        length: bytes.len() as u64,
        committed_length: offset + bytes.len() as u64,
        checksum: crc32fast::hash(&bytes),
    };
    file.seek(SeekFrom::Start(
        8 + ((generation - 1) % 2) * SLOT_SIZE as u64,
    ))?;
    file.write_all(&revision.encode())?;
    file.sync_all()?;
    progress.report(&crate::i18n::message("project.saved"), 1.);
    Ok(Archive {
        path: destination.to_path_buf(),
        entries: document.entries,
        revision: Some(revision),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Asset, SAMPLE_RATE};

    #[test]
    fn offsets_above_four_gib_remain_seekable() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("sparse.justspeak");
        let mut project = Project::new();
        let asset = Asset {
            id: crate::model::id(),
            name: "large.wav".into(),
            kind: "audio".into(),
            size: 4,
            duration: 1. / SAMPLE_RATE as f64,
            frames: 1,
            sample_rate: SAMPLE_RATE,
            waveform: vec![[0., 0.]],
            peak_frames: 1,
        };
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)?;
        file.write_all(&MAGIC)?;
        file.write_all(&[0; SLOT_SIZE * 2])?;
        let offset = u32::MAX as u64 + 1024;
        file.seek(SeekFrom::Start(offset))?;
        file.write_all(b"wave")?;
        file.write_all(&[1; 8])?;
        let mut entries = BTreeMap::new();
        entries.insert(format!("media/{}", asset.id), Region { offset, length: 4 });
        entries.insert(
            format!("cache/{}.pcm", asset.id),
            Region {
                offset: offset + 4,
                length: 8,
            },
        );
        project.assets.push(asset.clone());
        let source = Archive {
            path: path.clone(),
            entries,
            revision: Some(Revision {
                generation: 1,
                offset: 0,
                length: 0,
                committed_length: 0,
                checksum: 0,
            }),
        };
        let archive = write_revision(
            &mut file,
            &project,
            temp.path(),
            Some(&source),
            true,
            &path,
            &Progress::default(),
        )?;
        drop(file);
        let (_, reopened) = Archive::open(&path)?;
        let region = reopened
            .entries
            .get(&format!("cache/{}.pcm", asset.id))
            .unwrap();
        assert_eq!(region.offset, offset + 4);
        assert_eq!(archive.revision, reopened.revision);
        let mut file = File::open(&path)?;
        file.seek(SeekFrom::Start(region.offset))?;
        let mut bytes = [0; 8];
        file.read_exact(&mut bytes)?;
        assert_eq!(bytes, [1; 8]);
        Ok(())
    }
}
