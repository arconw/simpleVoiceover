use crate::model::{Effects, Project, id};
use crate::{archive::Archive, filesystem::atomic_json, indexed_archive, progress::Progress};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
    sync::Mutex,
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub working_directory: PathBuf,
    pub active_project: String,
    pub project_file: Option<PathBuf>,
    pub dirty: bool,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default)]
    pub effect_presets: Vec<EffectPreset>,
    #[serde(default)]
    pub input_device: String,
    #[serde(default)]
    pub output_device: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectPreset {
    pub id: String,
    pub name: String,
    pub effects: Effects,
    pub fx_bypass: bool,
}

fn default_language() -> String {
    "system".into()
}
pub struct Store {
    pub config_directory: PathBuf,
    pub config: Config,
    pub project: Project,
    pub history: Vec<Project>,
    pub redo: Vec<Project>,
    pub archive: Option<Archive>,
    pub progress: Progress,
    pub loudness_cache: Mutex<crate::loudness::LoudnessCache>,
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
                language: default_language(),
                effect_presets: vec![],
                input_device: String::new(),
                output_device: String::new(),
            }
        };
        if let Some(path) = working {
            config.working_directory = path;
        }
        uuid::Uuid::parse_str(&config.active_project)?;
        if config.language != "system"
            && !crate::i18n::LANGUAGES.contains(&config.language.as_str())
        {
            config.language = default_language();
        }
        ensure!(
            config.working_directory.is_absolute(),
            crate::i18n::message("error.workingDirectoryAbsolute")
        );
        let saved = config
            .project_file
            .as_deref()
            .map(Archive::open)
            .transpose()?;
        let active = project_root(&config).join("project.json");
        let mut project = if config.dirty && active.exists() {
            serde_json::from_reader(File::open(active)?.take(32 * 1024 * 1024))?
        } else if let Some((saved_project, _)) = &saved {
            saved_project.clone()
        } else {
            project
        };
        project.validate()?;
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
            archive: saved.map(|(_, archive)| archive),
            progress: Progress::default(),
            loudness_cache: Mutex::new(Default::default()),
        };
        store.persist()?;
        Ok(store)
    }
    pub fn root(&self) -> PathBuf {
        project_root(&self.config)
    }
    pub fn set_language(&mut self, language: &str) -> Result<()> {
        ensure!(
            language == "system" || crate::i18n::LANGUAGES.contains(&language),
            crate::i18n::message("error.language")
        );
        let mut config = self.config.clone();
        config.language = language.into();
        atomic_json(&self.config_directory.join("settings.json"), &config)
            .context(crate::i18n::message("error.preferencesSave"))?;
        self.config = config;
        Ok(())
    }
    pub fn set_audio_devices(&mut self, input: &str, output: &str) -> Result<()> {
        for device in [input, output] {
            ensure!(
                device.len() <= 4096 && !device.chars().any(char::is_control),
                crate::i18n::message("error.audioDevice")
            );
        }
        let mut config = self.config.clone();
        config.input_device = input.into();
        config.output_device = output.into();
        atomic_json(&self.config_directory.join("settings.json"), &config)
            .context(crate::i18n::message("error.preferencesSave"))?;
        self.config = config;
        Ok(())
    }
    pub fn save_preset(&mut self, name: &str, effects: Effects, fx_bypass: bool) -> Result<()> {
        let name = name.trim();
        ensure!(
            !name.is_empty() && name.chars().count() <= 80 && !name.chars().any(char::is_control),
            crate::i18n::message("error.presetName")
        );
        effects.validate()?;
        let mut config = self.config.clone();
        if let Some(preset) = config
            .effect_presets
            .iter_mut()
            .find(|preset| preset.name.to_lowercase() == name.to_lowercase())
        {
            preset.name = name.into();
            preset.effects = effects;
            preset.fx_bypass = fx_bypass;
        } else {
            ensure!(
                config.effect_presets.len() < 64,
                crate::i18n::message("error.presetLimit")
            );
            config.effect_presets.push(EffectPreset {
                id: id(),
                name: name.into(),
                effects,
                fx_bypass,
            });
        }
        atomic_json(&self.config_directory.join("settings.json"), &config)
            .context(crate::i18n::message("error.preferencesSave"))?;
        self.config = config;
        Ok(())
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
        self.begin_changes()?;
        self.history.push(self.project.clone());
        self.redo.clear();
        if self.history.len() > 40 {
            self.history.remove(0);
        }
        self.project = project;
        self.persist()
    }
    pub fn change_directory(&mut self, path: PathBuf) -> Result<()> {
        ensure!(
            path.is_absolute(),
            crate::i18n::message("error.workingDirectoryAbsolute")
        );
        fs::create_dir_all(&path).context(crate::i18n::message("error.createWorkingDirectory"))?;
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
            crate::i18n::message("error.workingDirectoryInsideCache")
        );
        ensure!(
            !new_root.exists(),
            crate::i18n::message("error.existingCache")
        );
        copy_directory(&old_root, &new_root)?;
        self.config.working_directory = path;
        self.persist()?;
        ensure!(
            old_root.file_name().and_then(|x| x.to_str()) == Some(self.project.id.as_str()),
            crate::i18n::message("error.oldCachePath")
        );
        fs::remove_dir_all(old_root)?;
        Ok(())
    }
    pub fn begin_changes(&mut self) -> Result<()> {
        if self.config.dirty {
            return Ok(());
        }
        self.config.dirty = true;
        self.persist()
    }
    pub fn location(&self, asset: &str, pcm: bool) -> Result<(PathBuf, u64, u64)> {
        let name = if pcm {
            format!("cache/{asset}.pcm")
        } else {
            format!("media/{asset}")
        };
        let path = self.root().join(&name);
        if self.config.dirty && path.exists() {
            let length = fs::metadata(&path)?.len();
            return Ok((path, 0, length));
        }
        let archive = self
            .archive
            .as_ref()
            .context(crate::i18n::message("error.projectFileMissing"))?;
        let region = archive
            .entries
            .get(&name)
            .context(crate::i18n::message("error.projectMediaMissing"))?;
        Ok((archive.path.clone(), region.offset, region.length))
    }
    pub fn save(&mut self, destination: &Path) -> Result<()> {
        self.save_with_options(destination, false)
    }
    pub fn save_with_options(&mut self, destination: &Path, compact: bool) -> Result<()> {
        if !compact
            && !self.config.dirty
            && self.config.project_file.as_deref() == Some(destination)
        {
            self.history.clear();
            self.redo.clear();
            return Ok(());
        }
        let old_root = self.root();
        let parent = destination
            .parent()
            .context(crate::i18n::message("error.projectPath"))?;
        ensure!(
            destination.is_absolute() && !destination.starts_with(&old_root),
            crate::i18n::message("error.saveOutsideCache")
        );
        fs::create_dir_all(parent)?;
        let archive = indexed_archive::save(
            &self.project,
            &old_root,
            self.archive.as_ref(),
            destination,
            compact,
            &self.progress,
        )?;
        self.archive = Some(archive);
        self.project.version = 3;
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
            crate::i18n::message("error.cachePath")
        );
        if old_root.exists() {
            fs::remove_dir_all(&old_root).context(crate::i18n::message("error.cacheCleanup"))?;
        }
        Ok(())
    }
    pub fn load(&mut self, archive: &Path) -> Result<()> {
        ensure!(
            archive.is_absolute(),
            crate::i18n::message("error.projectPath")
        );
        let (mut project, index) = Archive::open(archive)?;
        project.id = crate::model::id();
        self.project = project;
        self.config.active_project = self.project.id.clone();
        self.config.project_file = Some(archive.to_path_buf());
        self.config.dirty = false;
        self.archive = Some(index);
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
            crate::i18n::message("error.cachePath")
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
            crate::i18n::message("error.cacheSymlinks")
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
    use std::io::{Seek, SeekFrom};

    fn pcm_bytes(store: &Store, asset: &str) -> Result<Vec<u8>> {
        let (path, offset, length) = store.location(asset, true)?;
        let mut file = File::open(path)?;
        file.seek(SeekFrom::Start(offset))?;
        let mut bytes = Vec::new();
        file.take(length).read_to_end(&mut bytes)?;
        Ok(bytes)
    }
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
        store.begin_changes()?;
        assert!(store.root().starts_with(temp.path()));
        assert!(!store.pcm(&asset.id).exists());
        assert_eq!(pcm_bytes(&store, &asset.id)?, [1u8; 8]);
        store.save(&saved)?;
        assert!(!store.root().exists());
        let mut restored = Store::open(
            temp.path().join("elsewhere-config"),
            Some(temp.path().join("elsewhere")),
        )?;
        restored.load(&saved)?;
        restored.begin_changes()?;
        assert_eq!(restored.project.assets[0].waveform[0], [-0.5, 0.5]);
        assert!(!restored.pcm(&asset.id).exists());
        assert_eq!(pcm_bytes(&restored, &asset.id)?, [1u8; 8]);
        Ok(())
    }
}
