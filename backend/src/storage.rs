use crate::model::Project;
use crate::{
    archive::{extract_archive, read_manifest, save_archive, verify_archive},
    filesystem::atomic_json,
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
};
use zip::ZipArchive;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub working_directory: PathBuf,
    pub active_project: String,
    pub project_file: Option<PathBuf>,
    pub dirty: bool,
}
pub struct Store {
    pub config_directory: PathBuf,
    pub config: Config,
    pub project: Project,
    pub history: Vec<Project>,
    pub redo: Vec<Project>,
}

impl Store {
    pub fn open(config_directory: PathBuf, working: Option<PathBuf>) -> Result<Self> {
        fs::create_dir_all(&config_directory)?;
        let config_file = config_directory.join("settings.json");
        let project = Project::new();
        let mut config = if config_file.exists() {
            serde_json::from_reader(File::open(config_file)?)?
        } else {
            Config {
                working_directory: std::env::temp_dir().join("simpleVoiceover"),
                active_project: project.id.clone(),
                project_file: None,
                dirty: true,
            }
        };
        if let Some(path) = working {
            config.working_directory = path;
        }
        uuid::Uuid::parse_str(&config.active_project)?;
        ensure!(
            config.working_directory.is_absolute(),
            "Рабочий каталог должен быть абсолютным"
        );
        let active = project_root(&config).join("project.json");
        let mut project = if config.dirty && active.exists() {
            serde_json::from_reader(File::open(active)?.take(32 * 1024 * 1024))?
        } else if let Some(path) = &config.project_file {
            read_manifest(path)?
        } else {
            project
        };
        project.validate()?;
        ensure!(!project.tracks.is_empty(), "В проекте нет дорожек");
        if !config.dirty {
            project.id = config.active_project.clone();
        }
        config.active_project = project.id.clone();
        let store = Self {
            config_directory,
            config,
            project,
            history: vec![],
            redo: vec![],
        };
        store.persist()?;
        Ok(store)
    }
    pub fn root(&self) -> PathBuf {
        project_root(&self.config)
    }
    pub fn source(&self, id: &str) -> PathBuf {
        self.root().join("media").join(id)
    }
    pub fn pcm(&self, id: &str) -> PathBuf {
        self.root().join("cache").join(format!("{id}.pcm"))
    }
    pub fn persist(&self) -> Result<()> {
        if self.config.dirty {
            for dir in ["media", "cache"] {
                fs::create_dir_all(self.root().join(dir))?;
            }
            atomic_json(&self.root().join("project.json"), &self.project)?;
        }
        atomic_json(&self.config_directory.join("settings.json"), &self.config)?;
        Ok(())
    }
    pub fn replace(&mut self, project: Project) -> Result<()> {
        project.validate()?;
        self.materialize()?;
        self.history.push(self.project.clone());
        self.redo.clear();
        if self.history.len() > 40 {
            self.history.remove(0);
        }
        self.project = project;
        self.persist()
    }
    pub fn change_directory(&mut self, path: PathBuf) -> Result<()> {
        ensure!(path.is_absolute(), "Рабочий каталог должен быть абсолютным");
        fs::create_dir_all(&path).context("Не удалось создать рабочий каталог")?;
        let path = fs::canonicalize(path)?;
        if self.config.project_file.is_some() {
            self.config.working_directory = path;
            return self.persist();
        }
        let old_root = fs::canonicalize(self.root())?;
        let new_root = path.join(&self.project.id);
        if new_root == old_root {
            return Ok(());
        }
        ensure!(
            !path.starts_with(&old_root),
            "Нельзя выбрать папку внутри текущего кэша проекта"
        );
        ensure!(
            !new_root.exists(),
            "В выбранном каталоге уже есть кэш этого проекта"
        );
        copy_directory(&old_root, &new_root)?;
        self.config.working_directory = path;
        self.persist()?;
        ensure!(
            old_root.file_name().and_then(|x| x.to_str()) == Some(self.project.id.as_str()),
            "Небезопасный путь старого кэша"
        );
        fs::remove_dir_all(old_root)?;
        Ok(())
    }
    pub fn materialize(&mut self) -> Result<()> {
        if self.config.dirty {
            return Ok(());
        }
        let archive = self
            .config
            .project_file
            .as_ref()
            .context("Нет файла проекта")?;
        extract_archive(archive, &self.project, &self.root())?;
        self.config.dirty = true;
        self.persist()
    }
    pub fn location(&self, asset: &str, pcm: bool) -> Result<(PathBuf, u64, u64)> {
        let name = if pcm {
            format!("cache/{asset}.pcm")
        } else {
            format!("media/{asset}")
        };
        if self.config.dirty {
            let path = self.root().join(name);
            let length = fs::metadata(&path)?.len();
            return Ok((path, 0, length));
        }
        let path = self
            .config
            .project_file
            .as_ref()
            .context("Нет файла проекта")?;
        let mut archive = ZipArchive::new(File::open(path)?)?;
        let entry = archive.by_name(&name)?;
        Ok((path.clone(), entry.data_start(), entry.size()))
    }
    pub fn save(&mut self, destination: &Path) -> Result<()> {
        if !self.config.dirty && self.config.project_file.as_deref() == Some(destination) {
            self.history.clear();
            self.redo.clear();
            return Ok(());
        }
        self.materialize()?;
        let old_root = self.root();
        let parent = destination
            .parent()
            .context("Нужен абсолютный путь проекта")?;
        ensure!(
            destination.is_absolute() && !destination.starts_with(&old_root),
            "Сохрани проект вне его временного кэша"
        );
        fs::create_dir_all(parent)?;
        save_archive(&self.project, &old_root, destination)?;
        self.config.project_file = Some(destination.to_path_buf());
        self.config.dirty = false;
        self.history.clear();
        self.redo.clear();
        self.persist()?;
        ensure!(
            old_root
                .file_name()
                .and_then(|x| x.to_str())
                .is_some_and(|x| x == self.project.id
                    || x == format!(".simpleVoiceover-{}", self.project.id)),
            "Небезопасный путь кэша"
        );
        fs::remove_dir_all(&old_root)
            .context("Проект сохранён, но временный кэш не удалось убрать")?;
        Ok(())
    }
    pub fn load(&mut self, archive: &Path) -> Result<()> {
        let mut project = read_manifest(archive)?;
        verify_archive(archive, &project)?;
        project.id = crate::model::id();
        self.project = project;
        self.config.active_project = self.project.id.clone();
        self.config.project_file = Some(archive.to_path_buf());
        self.config.dirty = false;
        self.history.clear();
        self.redo.clear();
        self.persist()
    }
    pub fn discard(&mut self) -> Result<()> {
        if !self.config.dirty {
            return Ok(());
        }
        let old_root = self.root();
        let old_id = self.project.id.clone();
        if let Some(file) = self.config.project_file.clone() {
            self.load(&file)?;
        } else {
            self.project = Project::new();
            self.config.active_project = self.project.id.clone();
            self.history.clear();
            self.redo.clear();
            self.persist()?;
        }
        ensure!(
            old_root
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name == old_id || name == format!(".simpleVoiceover-{old_id}")),
            "Небезопасный путь кэша"
        );
        if old_root.exists() {
            fs::remove_dir_all(old_root)?;
        }
        Ok(())
    }
}

fn project_root(config: &Config) -> PathBuf {
    if let Some(file) = &config.project_file {
        file.parent()
            .unwrap()
            .join(format!(".simpleVoiceover-{}", config.active_project))
    } else {
        config.working_directory.join(&config.active_project)
    }
}

fn copy_directory(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        ensure!(
            !entry.file_type()?.is_symlink(),
            "Ссылки в кэше не поддерживаются"
        );
        let target = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_directory(&entry.path(), &target)?;
        } else {
            let mut input = File::open(entry.path())?;
            let mut output = File::create(target)?;
            io::copy(&mut input, &mut output)?;
            output.sync_all()?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Asset, SAMPLE_RATE};
    #[test]
    fn project_carries_cache_across_workspaces() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let mut store = Store::open(temp.path().join("config"), Some(temp.path().join("work")))?;
        let asset = Asset {
            id: crate::model::id(),
            name: "source.wav".into(),
            kind: "audio".into(),
            size: 4,
            duration: 1. / SAMPLE_RATE as f64,
            frames: 1,
            sample_rate: SAMPLE_RATE,
            waveform: vec![[-0.5, 0.5]],
            peak_frames: 1,
        };
        fs::write(store.source(&asset.id), b"wave")?;
        fs::write(store.pcm(&asset.id), [1u8; 8])?;
        store.project.assets.push(asset.clone());
        store.persist()?;
        let old = store.root();
        store.change_directory(temp.path().join("chosen"))?;
        assert!(!old.exists());
        assert_eq!(fs::read(store.pcm(&asset.id))?, [1u8; 8]);
        let saved = temp.path().join("session.justspeak");
        store.save(&saved)?;
        assert!(!store.root().exists());
        assert!(!store.config.dirty);
        let (_, offset, length) = store.location(&asset.id, true)?;
        assert!(offset > 0);
        assert_eq!(length, 8);
        store.materialize()?;
        assert!(store.root().starts_with(temp.path()));
        assert_eq!(fs::read(store.pcm(&asset.id))?, [1u8; 8]);
        store.save(&saved)?;
        assert!(!store.root().exists());
        let mut restored = Store::open(
            temp.path().join("elsewhere-config"),
            Some(temp.path().join("elsewhere")),
        )?;
        restored.load(&saved)?;
        restored.materialize()?;
        assert_eq!(restored.project.assets[0].waveform[0], [-0.5, 0.5]);
        assert_eq!(fs::read(restored.pcm(&asset.id))?, [1u8; 8]);
        Ok(())
    }
}
