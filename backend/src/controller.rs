use crate::{commands, mixer::Mixer, model::SAMPLE_RATE, session::AudioSession, storage::Store};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};

pub fn execute(store: &mut Store, session: &mut AudioSession, value: &Value) -> Result<Value> {
    let command = value["command"].as_str().context("Нет команды")?;
    if command != "snapshot" {
        tracing::info!(command, "Команда студии");
    }
    let position = value["position"].as_f64().unwrap_or(0.);
    ensure!(
        position.is_finite() && (0. ..=2_000_000.).contains(&position),
        "Некорректная позиция"
    );
    let result = match command {
        "play" => {
            ensure!(session.recording.is_none(), "Идёт запись");
            session.mixer = Some(Mixer::new(store)?);
            session.frame = (position * SAMPLE_RATE as f64).round() as u64;
            session.playing = true;
            json!({"playing":true})
        }
        "pause" => {
            ensure!(session.recording.is_none(), "Сначала заверши запись");
            session.playing = false;
            session.mixer = None;
            json!({"playing":false})
        }
        "monitor" => {
            session.monitor = value["enabled"].as_bool().unwrap_or(false);
            session.input.clear();
            json!({"monitor":session.monitor})
        }
        "record_begin" => {
            session.begin_record(store, position)?;
            json!({"recording":true,"snapshot":commands::snapshot(store)})
        }
        "record_end" => {
            session.finish_record(store)?;
            commands::snapshot(store)
        }
        _ => {
            ensure!(
                session.recording.is_none() || ["track_patch", "snapshot"].contains(&command),
                "Сначала заверши запись"
            );
            if session.recording.is_some() {
                ensure!(
                    value["patch"]
                        .as_object()
                        .is_none_or(|patch| !patch.keys().any(|key| key == "armed")),
                    "Нельзя менять дорожку записи во время дубля"
                );
            }
            if !["track_patch", "snapshot"].contains(&command) {
                session.playing = false;
                session.mixer = None;
            }
            commands::execute(store, value)?
        }
    };
    Ok(result)
}
