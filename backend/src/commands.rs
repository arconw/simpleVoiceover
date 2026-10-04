use crate::{
    media, mixer,
    model::{Asset, Clip, MAX_UPLOAD, SAMPLE_RATE, Track, id},
    native_files,
    storage::Store,
};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{
    fs::{self, File},
    path::PathBuf,
};

pub fn snapshot(store: &Store) -> Value {
    json!({"project":store.project,"config":store.config,"duration":store.project.duration(),"canUndo":!store.history.is_empty(),"canRedo":!store.redo.is_empty()})
}
pub(crate) fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value[key].as_str().with_context(|| format!("Нет {key}"))
}
pub(crate) fn number(value: &Value, key: &str) -> Result<f64> {
    let n = value[key].as_f64().with_context(|| format!("Нет {key}"))?;
    ensure!(n.is_finite(), "Некорректное число");
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
    ensure!(size <= MAX_UPLOAD, "Лимит файла — 100 ГБ");
    store.materialize()?;
    let asset_id = id();
    let owned = store.source(&asset_id);
    let mut input = File::open(&source)?;
    let mut output = File::create(&owned)?;
    std::io::copy(&mut input, &mut output)?;
    output.sync_all()?;
    let asset = media::decode(
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
    )?;
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
            project.tracks.push(Track::new(
                &kind,
                if kind == "video" {
                    "Звук видео"
                } else {
                    "Аудио"
                },
            ));
            project.tracks.len() - 1
        });
    let track = &mut project.tracks[index];
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
    store.replace(project)
}

pub fn execute(store: &mut Store, value: &Value) -> Result<Value> {
    let command = text(value, "command")?;
    match command {
        "snapshot" => {}
        "working_directory" => {
            let path = if let Some(path) = value["path"].as_str() {
                Some(PathBuf::from(path))
            } else {
                native_files::directory()?
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
                native_files::project(true)?
            };
            if let Some(destination) = destination {
                store.save(&destination)?;
                return Ok(json!({"saved":true,"path":destination,"snapshot":snapshot(store)}));
            }
            return Ok(json!({"saved":false}));
        }
        "open" => {
            let source = if let Some(path) = value["path"].as_str() {
                Some(PathBuf::from(path))
            } else {
                native_files::project(false)?
            };
            if let Some(source) = source {
                store.load(&source)?;
            }
        }
        "import_native" | "import_paths" => {
            let paths = if command == "import_native" {
                native_files::media()?.unwrap_or_default()
            } else {
                value["paths"]
                    .as_array()
                    .context("Нет файлов")?
                    .iter()
                    .map(|path| path.as_str().map(PathBuf::from).context("Неверный путь"))
                    .collect::<Result<Vec<_>>>()?
            };
            ensure!(paths.len() <= 128, "Слишком много файлов за один импорт");
            for source in paths {
                let name = source
                    .file_name()
                    .context("Нет имени файла")?
                    .to_string_lossy()
                    .into_owned();
                let extension = source
                    .extension()
                    .and_then(|x| x.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                let kind = if ["mp4", "mov", "mkv", "webm", "m4v"].contains(&extension.as_str()) {
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
        }
        "export" => {
            let format = text(value, "format")?;
            ensure!(["wav", "mp3"].contains(&format), "Неверный формат");
            let destination = if let Some(path) = value["path"].as_str() {
                Some(PathBuf::from(path))
            } else {
                native_files::export(format)?
            };
            let Some(destination) = destination else {
                return Ok(json!({"saved":false}));
            };
            ensure!(destination.is_absolute(), "Нужен абсолютный путь экспорта");
            ensure!(
                store.config.project_file.as_ref() != Some(&destination)
                    && !destination.starts_with(store.root()),
                "Нельзя перезаписать проект или его кэш экспортом"
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
                store.materialize()?;
                store.redo.push(store.project.clone());
                store.project = project;
                store.persist()?;
            }
        }
        "redo" => {
            if let Some(project) = store.redo.pop() {
                store.materialize()?;
                store.history.push(store.project.clone());
                store.project = project;
                store.persist()?;
            }
        }
        _ => crate::editing::execute(store, value)?,
    }
    Ok(snapshot(store))
}
