use crate::commands::{number, text};
use crate::{
    model::{Clip, SAMPLE_RATE, Track, id},
    storage::Store,
};
use anyhow::{Context, Result, bail, ensure};
use serde_json::Value;

pub fn execute(store: &mut Store, value: &Value) -> Result<()> {
    let command = text(value, "command")?;
    match command {
        "asset_remove" => {
            let asset_id = text(value, "assetId")?;
            let mut project = store.project.clone();
            ensure!(
                project.assets.iter().any(|asset| asset.id == asset_id),
                crate::i18n::message("error.sourceMissing")
            );
            ensure!(
                !project.tracks.iter().any(|track| track.locked
                    && track.clips.iter().any(|clip| clip.asset_id == asset_id)),
                crate::i18n::message("error.assetOnLockedTrack")
            );
            for track in &mut project.tracks {
                track.clips.retain(|clip| clip.asset_id != asset_id);
            }
            project.assets.retain(|asset| asset.id != asset_id);
            store.replace(project)?;
        }
        "track_add" => {
            let mut project = store.project.clone();
            project.tracks.push(Track::new("audio", "track.audio"));
            store.replace(project)?;
        }
        "track_remove" => {
            let track_id = text(value, "trackId")?;
            let mut project = store.project.clone();
            let index = project
                .tracks
                .iter()
                .position(|track| track.id == track_id)
                .context(crate::i18n::message("error.trackMissing"))?;
            ensure!(
                !project.tracks[index].locked,
                crate::i18n::message("error.trackLocked")
            );
            project.tracks.remove(index);
            store.replace(project)?;
        }
        "track_patch" => {
            let track_id = text(value, "trackId")?;
            let mut project = store.project.clone();
            let index = project
                .tracks
                .iter()
                .position(|t| t.id == track_id)
                .context(crate::i18n::message("error.trackMissing"))?;
            let patch = value["patch"]
                .as_object()
                .context(crate::i18n::message("error.patchMissing"))?;
            let mut serialized = serde_json::to_value(&project.tracks[index])?;
            for (key, val) in patch {
                ensure!(
                    [
                        "name", "mute", "solo", "locked", "armed", "fxBypass", "volume", "pan",
                        "effects"
                    ]
                    .contains(&key.as_str()),
                    crate::i18n::message("error.trackField")
                );
                serialized[key] = val.clone();
            }
            let updated: Track = serde_json::from_value(serialized)?;
            if updated.armed {
                for t in &mut project.tracks {
                    t.armed = false;
                }
            }
            project.tracks[index] = updated;
            store.replace(project)?;
        }
        "clip_place" => {
            let mut project = store.project.clone();
            let asset = project
                .assets
                .iter()
                .find(|a| Some(a.id.as_str()) == value["assetId"].as_str())
                .context(crate::i18n::message("error.sourceMissing"))?
                .clone();
            let track = project
                .tracks
                .iter_mut()
                .find(|t| Some(t.id.as_str()) == value["trackId"].as_str())
                .context(crate::i18n::message("error.trackMissing"))?;
            ensure!(
                !track.locked && (asset.kind == "video") == (track.kind == "video"),
                crate::i18n::message("error.selectCompatibleTrack")
            );
            track.clips.push(Clip {
                id: id(),
                asset_id: asset.id,
                name: asset.name,
                start: number(value, "position")?.max(0.),
                offset: 0.,
                duration: asset.duration,
            });
            store.replace(project)?;
        }
        "clip_edit" | "clip_split" | "clip_remove" => {
            let mut project = store.project.clone();
            let assets = project.assets.clone();
            let track = project
                .tracks
                .iter_mut()
                .find(|t| Some(t.id.as_str()) == value["trackId"].as_str())
                .context(crate::i18n::message("error.trackMissing"))?;
            ensure!(!track.locked, crate::i18n::message("error.trackLocked"));
            let index = track
                .clips
                .iter()
                .position(|c| Some(c.id.as_str()) == value["clipId"].as_str())
                .context(crate::i18n::message("error.clipMissing"))?;
            if command == "clip_remove" {
                track.clips.remove(index);
            } else if command == "clip_split" {
                let original = track.clips[index].clone();
                let cut = (number(value, "position")? * SAMPLE_RATE as f64).round()
                    / SAMPLE_RATE as f64
                    - original.start;
                if cut >= 1. / SAMPLE_RATE as f64
                    && original.duration - cut >= 1. / SAMPLE_RATE as f64
                {
                    track.clips[index].duration = cut;
                    track.clips.insert(
                        index + 1,
                        Clip {
                            id: id(),
                            start: original.start + cut,
                            offset: original.offset + cut,
                            duration: original.duration - cut,
                            ..original
                        },
                    );
                }
            } else {
                let clip = &mut track.clips[index];
                let delta = number(value, "delta")?;
                match text(value, "mode")? {
                    "move" => clip.start = (clip.start + delta).max(0.),
                    "left" => {
                        let d = delta.clamp(
                            -clip.start.min(clip.offset),
                            (clip.duration - 1. / SAMPLE_RATE as f64).max(0.),
                        );
                        clip.start += d;
                        clip.offset += d;
                        clip.duration -= d;
                    }
                    "right" => {
                        let duration = assets
                            .iter()
                            .find(|a| a.id == clip.asset_id)
                            .unwrap()
                            .duration;
                        clip.duration = (clip.duration + delta)
                            .clamp(1. / SAMPLE_RATE as f64, duration - clip.offset);
                    }
                    _ => bail!(crate::i18n::message("error.editTool")),
                }
            }
            store.replace(project)?;
        }
        _ => bail!(crate::i18n::message("error.editCommand")),
    }
    Ok(())
}
