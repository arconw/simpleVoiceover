use crate::{session::AudioSession, storage::Store};
use anyhow::{Result, ensure};

pub fn prepare_close(store: &mut Store, audio: &AudioSession, discard: bool) -> Result<()> {
    ensure!(
        audio.recording.is_none(),
        crate::i18n::message("error.finishRecording")
    );
    ensure!(
        !store.config.dirty || discard,
        crate::i18n::message("error.saveChangesFirst")
    );
    if discard {
        store.discard()?;
    }
    Ok(())
}
