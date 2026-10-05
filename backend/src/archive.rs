use crate::{indexed_archive, model::Project};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};
use zip::{CompressionMethod, ZipArchive};

pub const MAX_MANIFEST: u64 = 32 * 1024 * 1024;

#[derive(Clone, Serialize, Deserialize)]
pub struct Region {
    pub offset: u64,
    pub length: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Revision {
    pub generation: u64,
    pub offset: u64,
    pub length: u64,
    pub committed_length: u64,
    pub checksum: u32,
}

pub struct Archive {
    pub path: PathBuf,
    pub entries: BTreeMap<String, Region>,
    pub revision: Option<Revision>,
}

impl Archive {
    pub fn open(path: &Path) -> Result<(Project, Self)> {
        let mut file = File::open(path)?;
        let mut magic = [0; 8];
        file.read_exact(&mut magic)?;
        file.seek(SeekFrom::Start(0))?;
        if magic == indexed_archive::MAGIC {
            return indexed_archive::read(file, path);
        }
        let mut zip = ZipArchive::new(file)?;
        let project: Project =
            serde_json::from_reader(zip.by_name("project.json")?.take(MAX_MANIFEST))?;
        project.validate()?;
        ensure!(
            project.version == 2,
            crate::i18n::message("error.zipVersion")
        );
        let mut entries = BTreeMap::new();
        for (name, expected) in asset_entries(&project) {
            let entry = zip.by_name(&name)?;
            ensure!(
                entry.size() == expected && entry.compression() == CompressionMethod::Stored,
                crate::i18n::message("error.corruptMedia")
            );
            entries.insert(
                name,
                Region {
                    offset: entry.data_start(),
                    length: entry.size(),
                },
            );
        }
        ensure!(
            zip.len() == entries.len() + 1,
            crate::i18n::message("error.unexpectedFiles")
        );
        Ok((
            project,
            Self {
                path: path.to_path_buf(),
                entries,
                revision: None,
            },
        ))
    }
}

pub fn asset_entries(project: &Project) -> impl Iterator<Item = (String, u64)> + '_ {
    project.assets.iter().flat_map(|asset| {
        [
            (format!("media/{}", asset.id), asset.size),
            (format!("cache/{}.pcm", asset.id), asset.frames * 8),
        ]
    })
}
