use crate::{
    media, mixer,
    model::{Asset, Clip, MAX_UPLOAD, SAMPLE_RATE, Track, id},
    native_files,
    storage::Store,
};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn snapshot(store: &Store) -> Value {
    json!({"project":store.project,"config":store.config,"duration":store.project.duration(),"canUndo":!store.history.is_empty(),"canRedo":!store.redo.is_empty()})
}
pub(crate) fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value[key].as_str().with_context(|| {
        crate::i18n::formatted_message("error.fieldMissing", &[("key", key.into())])
    })
}
pub(crate) fn number(value: &Value, key: &str) -> Result<f64> {
    let n = value[key].as_f64().with_context(|| {
        crate::i18n::formatted_message("error.fieldMissing", &[("key", key.into())])
    })?;
    ensure!(n.is_finite(), crate::i18n::message("error.number"));
    Ok(n)
}

pub fn import_source(
    store: &mut Store,
    source: PathBuf,
    name: String,
    kind: String,
    target: Option<String>,
    position: f64,
) -> Result<()> {
    let size = fs::metadata(&source)?.len();
    ensure!(size <= MAX_UPLOAD, crate::i18n::message("error.fileLimit"));
    ensure!(
        position.is_finite() && position >= 0.,
        crate::i18n::message("error.position")
    );
    let was_dirty = store.config.dirty;
    let had_root = store.root().exists();
    store.begin_changes()?;
    let asset_id = id();
    let owned = store.source(&asset_id);
    let decoded = media::import(
        &source,
        &owned,
        &store.pcm(&asset_id),
        Asset {
            id: asset_id.clone(),
            name: name.clone(),
            kind: kind.clone(),
            size,
            duration: 0.,
            frames: 0,
            sample_rate: SAMPLE_RATE,
            waveform: vec![],
            peak_frames: 256,
        },
        &store.progress,
    );
    let assets = match decoded {
        Ok(assets) => assets,
        Err(error) => {
            let _ = fs::remove_file(&owned);
            let _ = fs::remove_file(store.pcm(&asset_id));
            if !was_dirty {
                store.config.dirty = false;
                store.persist()?;
                if !had_root {
                    let _ = fs::remove_dir_all(store.root());
                }
            }
            return Err(error);
        }
    };
    let imported_ids: Vec<_> = assets.iter().map(|asset| asset.id.clone()).collect();
    let result = (|| {
        let mut assets = assets.into_iter();
        let asset = assets
            .next()
            .context(crate::i18n::message("error.noDecodableAudio"))?;
        let mut project = store.project.clone();
        let index = project
            .tracks
            .iter()
            .position(|t| {
                !t.locked
                    && if kind == "video" {
                        t.kind == "video"
                    } else {
                        target.as_ref().is_some_and(|id| id == &t.id) && t.kind != "video"
                    }
            })
            .unwrap_or_else(|| {
                let track = Track::new(
                    &kind,
                    if kind == "video" {
                        "track.video"
                    } else {
                        "track.audio"
                    },
                );
                if kind == "video" {
                    project.tracks.insert(0, track);
                    0
                } else {
                    project.tracks.push(track);
                    project.tracks.len() - 1
                }
            });
        let track = &mut project.tracks[index];
        if kind == "video" && asset.frames == 0 && track.name == "track.video" {
            track.name = "track.videoPreview".into();
        }
        let start = track
            .clips
            .iter()
            .map(|c| c.start + c.duration)
            .fold(position, f64::max);
        track.clips.push(Clip {
            id: id(),
            asset_id,
            name,
            start,
            offset: 0.,
            duration: asset.duration,
        });
        project.assets.push(asset);
        for (stream_index, audio) in assets.enumerate() {
            let track_name = Path::new(&audio.name)
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy();
            let mut track = Track::new("audio", &track_name);
            track.mute = stream_index > 0;
            track.clips.push(Clip {
                id: id(),
                asset_id: audio.id.clone(),
                name: audio.name.clone(),
                start,
                offset: 0.,
                duration: audio.duration,
            });
            project.tracks.insert(index + stream_index + 1, track);
            project.assets.push(audio);
        }
        project.validate()?;
        Ok::<_, anyhow::Error>(project)
    })();
    if result.is_err() {
        for asset_id in imported_ids {
            let _ = fs::remove_file(store.source(&asset_id));
            let _ = fs::remove_file(store.pcm(&asset_id));
        }
        if !was_dirty {
            store.config.dirty = false;
            store.persist()?;
            if !had_root {
                let _ = fs::remove_dir_all(store.root());
            }
        }
    }
    store.replace(result?)
}

pub fn execute(store: &mut Store, value: &Value) -> Result<Value> {
    let command = text(value, "command")?;
    let language = value["language"].as_str().unwrap_or("en");
    match command {
        "snapshot" => {}
        "preferences_patch" => {
            store.set_language(text(value, "preference")?)?;
        }
        "audio_preferences_patch" => {
            store.set_audio_devices(text(value, "inputDevice")?, text(value, "outputDevice")?)?;
        }
        "preset_save" => {
            store.save_preset(
                text(value, "name")?,
                serde_json::from_value(value["effects"].clone())?,
                value["fxBypass"].as_bool().unwrap_or(false),
            )?;
        }
        "regions_edit" | "clips_paste" => return crate::regions::execute(store, value),
        "working_directory" => {
            let path = if let Some(path) = value["path"].as_str() {
                Some(PathBuf::from(path))
            } else {
                native_files::directory(language)?
            };
            if let Some(path) = path {
                store.change_directory(path)?;
            }
        }
        "save" => {
            let destination = if let Some(path) = value["path"].as_str() {
                Some(PathBuf::from(path))
            } else if !value["saveAs"].as_bool().unwrap_or(false)
                && store.config.project_file.is_some()
            {
                store.config.project_file.clone()
            } else {
                native_files::project(true, language)?
            };
            if let Some(destination) = destination {
                if value["saveAs"].as_bool().unwrap_or(false) {
                    store.save_with_options(&destination, true)?;
                } else {
                    store.save(&destination)?;
                }
                return Ok(json!({"saved":true,"path":destination,"snapshot":snapshot(store)}));
            }
            return Ok(json!({"saved":false}));
        }
        "open" => {
            let source = if let Some(path) = value["path"].as_str() {
                Some(PathBuf::from(path))
            } else {
                native_files::project(false, language)?
            };
            if let Some(source) = source {
                store.load(&source)?;
            }
        }
        "import_native" | "import_paths" => {
            let paths = if command == "import_native" {
                native_files::media(language)?.unwrap_or_default()
            } else {
                value["paths"]
                    .as_array()
                    .context(crate::i18n::message("error.filesMissing"))?
                    .iter()
                    .map(|path| {
                        path.as_str()
                            .map(PathBuf::from)
                            .context(crate::i18n::message("error.path"))
                    })
                    .collect::<Result<Vec<_>>>()?
            };
            ensure!(
                paths.len() <= 128,
                crate::i18n::message("error.importFileCount")
            );
            let progress = store.progress.clone();
            let count = paths.len();
            let imported = (|| {
                for (index, source) in paths.into_iter().enumerate() {
                    store.progress = progress.range(index as f64 / count as f64, 1. / count as f64);
                    let name = source
                        .file_name()
                        .context(crate::i18n::message("error.filenameMissing"))?
                        .to_string_lossy()
                        .into_owned();
                    let extension = source
                        .extension()
                        .and_then(|x| x.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    let kind = if ["mp4", "mov", "mkv", "webm", "m4v"].contains(&extension.as_str())
                    {
                        "video"
                    } else {
                        "audio"
                    };
                    import_source(
                        store,
                        source,
                        name,
                        kind.into(),
                        value["trackId"].as_str().map(str::to_owned),
                        number(value, "position")?,
                    )?;
                }
                Ok::<_, anyhow::Error>(())
            })();
            store.progress = progress;
            imported?;
        }
        "export" => {
            let format = text(value, "format")?;
            ensure!(
                ["wav", "mp3"].contains(&format),
                crate::i18n::message("error.format")
            );
            let name = if let Some(track_id) = value["trackId"].as_str() {
                store
                    .project
                    .tracks
                    .iter()
                    .find(|track| track.id == track_id)
                    .context(crate::i18n::message("error.trackMissing"))?
                    .name
                    .clone()
            } else {
                store
                    .config
                    .project_file
                    .as_deref()
                    .and_then(|path| path.file_stem())
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| store.project.name.clone())
            };
            let destination = if let Some(path) = value["path"].as_str() {
                Some(PathBuf::from(path))
            } else {
                native_files::export(format, language, &name)?
            };
            let Some(destination) = destination else {
                return Ok(json!({"saved":false}));
            };
            ensure!(
                destination.is_absolute(),
                crate::i18n::message("error.exportPath")
            );
            ensure!(
                store.config.project_file.as_ref() != Some(&destination)
                    && !destination.starts_with(store.root()),
                crate::i18n::message("error.exportOverwritesProject")
            );
            let temporary = destination.with_extension(format!("{}.partial", id()));
            let rendered = mixer::render(store, &temporary, value["trackId"].as_str(), format);
            if let Err(error) = rendered {
                let _ = fs::remove_file(&temporary);
                return Err(error);
            }
            crate::filesystem::replace_file(&temporary, &destination)?;
            return Ok(json!({"saved":true,"path":destination}));
        }
        "undo" => {
            if let Some(project) = store.history.pop() {
                store.begin_changes()?;
                store.redo.push(store.project.clone());
                store.project = project;
                store.persist()?;
            }
        }
        "redo" => {
            if let Some(project) = store.redo.pop() {
                store.begin_changes()?;
                store.history.push(store.project.clone());
                store.project = project;
                store.persist()?;
            }
        }
        _ => crate::editing::execute(store, value)?,
    }
    Ok(snapshot(store))
}
