use anyhow::Result;
#[cfg(not(feature = "desktop"))]
use anyhow::bail;
use std::path::PathBuf;

pub fn directory(language: &str) -> Result<Option<PathBuf>> {
    #[cfg(feature = "desktop")]
    {
        Ok(rfd::FileDialog::new()
            .set_title(crate::i18n::text(language, "dialog.workingDirectory"))
            .pick_folder())
    }
    #[cfg(not(feature = "desktop"))]
    {
        let _ = language;
        bail!(crate::i18n::message("error.desktopOnly"))
    }
}

pub fn project(save: bool, language: &str) -> Result<Option<PathBuf>> {
    #[cfg(feature = "desktop")]
    {
        let dialog = rfd::FileDialog::new()
            .add_filter("simpleVoiceover", &["justspeak"])
            .set_file_name(crate::i18n::text(language, "dialog.projectFilename"));
        Ok(if save {
            dialog.save_file()
        } else {
            dialog.pick_file()
        })
    }
    #[cfg(not(feature = "desktop"))]
    {
        let _ = (save, language);
        bail!(crate::i18n::message("error.desktopOnly"))
    }
}

pub fn media(language: &str) -> Result<Option<Vec<PathBuf>>> {
    #[cfg(feature = "desktop")]
    {
        Ok(rfd::FileDialog::new()
            .set_title(crate::i18n::text(language, "media.add"))
            .add_filter(
                crate::i18n::text(language, "dialog.mediaFilter"),
                &[
                    "mp4", "mov", "m4a", "wav", "mp3", "flac", "ogg", "mkv", "webm",
                ],
            )
            .pick_files())
    }
    #[cfg(not(feature = "desktop"))]
    {
        let _ = language;
        bail!(crate::i18n::message("error.desktopOnly"))
    }
}

pub fn export(format: &str, language: &str) -> Result<Option<PathBuf>> {
    #[cfg(feature = "desktop")]
    {
        Ok(rfd::FileDialog::new()
            .set_title(crate::i18n::text(language, "dialog.exportAudio"))
            .add_filter(format.to_uppercase(), &[format])
            .set_file_name(format!("simpleVoiceover.{format}"))
            .save_file())
    }
    #[cfg(not(feature = "desktop"))]
    {
        let _ = (format, language);
        bail!(crate::i18n::message("error.desktopOnly"))
    }
}
