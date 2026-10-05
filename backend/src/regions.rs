use crate::{
    commands::{number, text},
    model::{Clip, Project, SAMPLE_RATE, id},
    storage::Store,
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashSet;

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Region {
    track_id: String,
    clip_id: String,
    from: f64,
    to: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PasteClip {
    track_offset: usize,
    clip: Clip,
}

fn compatible(project: &Project, track: usize, clip: &Clip) -> Result<()> {
    ensure!(
        !project.tracks[track].locked,
        crate::i18n::message("error.trackLocked")
    );
    let asset = project
        .assets
        .iter()
        .find(|asset| asset.id == clip.asset_id)
        .context(crate::i18n::message("error.sourceMissing"))?;
    ensure!(
        (asset.kind == "video") == (project.tracks[track].kind == "video"),
        crate::i18n::message("error.selectCompatibleTrack")
    );
    Ok(())
}

pub fn execute(store: &mut Store, value: &Value) -> Result<Value> {
    let mut project = store.project.clone();
    let mut selected = vec![];
    match text(value, "command")? {
        "regions_edit" => {
            let regions: Vec<Region> = serde_json::from_value(value["regions"].clone())?;
            ensure!(
                !regions.is_empty() && regions.len() <= 10000,
                crate::i18n::message("error.selection")
            );
            let remove = value["remove"].as_bool().unwrap_or(false);
            let delta = number(value, "delta")?;
            let shift = value["trackOffset"]
                .as_i64()
                .context(crate::i18n::message("error.selection"))?;
            ensure!(
                (-127..=127).contains(&shift),
                crate::i18n::message("error.selection")
            );
            let mut seen = HashSet::new();
            let mut additions = vec![];
            for region in regions {
                ensure!(
                    seen.insert(region.clip_id.clone())
                        && region.from.is_finite()
                        && region.to.is_finite(),
                    crate::i18n::message("error.selection")
                );
                let source = project
                    .tracks
                    .iter()
                    .position(|track| track.id == region.track_id)
                    .context(crate::i18n::message("error.trackMissing"))?;
                ensure!(
                    !project.tracks[source].locked,
                    crate::i18n::message("error.trackLocked")
                );
                let index = project.tracks[source]
                    .clips
                    .iter()
                    .position(|clip| clip.id == region.clip_id)
                    .context(crate::i18n::message("error.clipMissing"))?;
                let original = project.tracks[source].clips[index].clone();
                let duration_frames = (original.duration * SAMPLE_RATE as f64).round();
                let first = ((region.from - original.start) * SAMPLE_RATE as f64).round();
                let last = ((region.to - original.start) * SAMPLE_RATE as f64).round();
                ensure!(
                    first >= 0. && last <= duration_frames && last - first >= 1.,
                    crate::i18n::message("error.selection")
                );
                let from = original.start + first / SAMPLE_RATE as f64;
                let to = (original.start + last / SAMPLE_RATE as f64)
                    .min(original.start + original.duration);
                let whole = first == 0. && last == duration_frames;
                if !remove {
                    let target = source as i64 + shift;
                    ensure!(
                        target >= 0 && target < project.tracks.len() as i64,
                        crate::i18n::message("error.trackMissing")
                    );
                    compatible(&project, target as usize, &original)?;
                    let moved = Clip {
                        id: if whole { original.id.clone() } else { id() },
                        start: from + delta,
                        offset: original.offset + from - original.start,
                        duration: to - from,
                        ..original.clone()
                    };
                    ensure!(moved.start >= 0., crate::i18n::message("error.clipBounds"));
                    selected.push(Region {
                        track_id: project.tracks[target as usize].id.clone(),
                        clip_id: moved.id.clone(),
                        from: moved.start,
                        to: moved.start + moved.duration,
                    });
                    additions.push((target as usize, moved));
                }
                let clips = &mut project.tracks[source].clips;
                clips.remove(index);
                if first > 0. {
                    clips.push(Clip {
                        duration: from - original.start,
                        ..original.clone()
                    });
                }
                if last < duration_frames {
                    clips.push(Clip {
                        id: if first > 0. {
                            id()
                        } else {
                            original.id.clone()
                        },
                        start: to,
                        offset: original.offset + to - original.start,
                        duration: original.start + original.duration - to,
                        ..original
                    });
                }
            }
            for (target, clip) in additions {
                project.tracks[target].clips.push(clip);
            }
        }
        "clips_paste" => {
            ensure!(
                value["projectId"].as_str() == Some(project.id.as_str()),
                crate::i18n::message("error.clipboardProject")
            );
            let clips: Vec<PasteClip> = serde_json::from_value(value["clips"].clone())?;
            ensure!(
                !clips.is_empty() && clips.len() <= 10000,
                crate::i18n::message("error.selection")
            );
            let first = project
                .tracks
                .iter()
                .position(|track| Some(track.id.as_str()) == value["trackId"].as_str())
                .context(crate::i18n::message("error.trackMissing"))?;
            let position = number(value, "position")?;
            for entry in clips {
                let target = first
                    .checked_add(entry.track_offset)
                    .filter(|index| *index < project.tracks.len())
                    .context(crate::i18n::message("error.trackMissing"))?;
                compatible(&project, target, &entry.clip)?;
                let clip = Clip {
                    id: id(),
                    start: position + entry.clip.start,
                    ..entry.clip
                };
                selected.push(Region {
                    track_id: project.tracks[target].id.clone(),
                    clip_id: clip.id.clone(),
                    from: clip.start,
                    to: clip.start + clip.duration,
                });
                project.tracks[target].clips.push(clip);
            }
        }
        _ => unreachable!(),
    }
    for track in &mut project.tracks {
        track
            .clips
            .sort_by(|left, right| left.start.total_cmp(&right.start));
    }
    store.replace(project)?;
    Ok(json!({"snapshot":crate::commands::snapshot(store), "regions":selected}))
}
