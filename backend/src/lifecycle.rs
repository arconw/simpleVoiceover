use crate::{session::AudioSession, storage::Store};
use anyhow::{Result, ensure};

pub fn prepare_close(store: &mut Store, audio: &AudioSession, discard: bool) -> Result<()> {
    ensure!(audio.recording.is_none(), "Сначала заверши запись");
    ensure!(!store.config.dirty || discard, "Сначала сохрани изменения");
    if discard {
        store.discard()?;
    }
    Ok(())
}
