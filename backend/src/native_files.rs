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

pub fn export_filename(name: &str, format: &str, language: &str) -> String {
    let name = if name.starts_with("track.") || name.starts_with("project.") {
        crate::i18n::text(language, name)
    } else {
        name.to_owned()
    };
    let sanitized: String = name
        .chars()
        .map(|character| {
            if character.is_control() || "<>:\"/\\|?*".contains(character) {
                '_'
            } else {
                character
            }
        })
        .collect();
    let mut base = sanitized
        .trim_matches(|character: char| character.is_whitespace() || character == '.')
        .to_owned();
    if base.is_empty() {
        base = crate::i18n::text(language, "project.defaultName");
    }
    let device = base
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    if ["CON", "PRN", "AUX", "NUL"].contains(&device.as_str())
        || ["COM", "LPT"].iter().any(|prefix| {
            device.strip_prefix(prefix).is_some_and(|suffix| {
                suffix.len() == 1 && matches!(suffix.as_bytes()[0], b'1'..=b'9')
            })
        })
    {
        base.insert(0, '_');
    }
    while base.len() > 200 {
        base.pop();
    }
    format!("{base}.{format}")
}

pub fn export(format: &str, language: &str, name: &str) -> Result<Option<PathBuf>> {
    #[cfg(feature = "desktop")]
    {
        Ok(rfd::FileDialog::new()
            .set_title(crate::i18n::text(language, "dialog.exportAudio"))
            .add_filter(format.to_uppercase(), &[format])
            .set_file_name(export_filename(name, format, language))
            .save_file())
    }
    #[cfg(not(feature = "desktop"))]
    {
        let _ = (format, language, name);
        bail!(crate::i18n::message("error.desktopOnly"))
    }
}

#[cfg(test)]
mod tests {
    use super::export_filename;

    #[test]
    fn export_names_use_localized_track_names_and_preserve_custom_names() {
        assert_eq!(export_filename("track.voice", "mp3", "en"), "My voice.mp3");
        assert_eq!(
            export_filename("track.voice", "wav", "ru"),
            format!("{}.wav", crate::i18n::text("ru", "track.voice"))
        );
        assert_eq!(
            export_filename("Narration 02", "mp3", "en"),
            "Narration 02.mp3"
        );
        assert_eq!(export_filename("日本語", "wav", "ja"), "日本語.wav");
    }

    #[test]
    fn export_names_are_valid_on_windows_and_linux_and_bound_unicode_length() {
        assert_eq!(
            export_filename(" ../intro\\take:01? ", "mp3", "en"),
            "_intro_take_01_.mp3"
        );
        assert_eq!(export_filename("CON", "wav", "en"), "_CON.wav");
        assert_eq!(export_filename("lpt9.part", "wav", "en"), "_lpt9.part.wav");
        assert_eq!(export_filename(" ... ", "wav", "en"), "My session.wav");
        let name = export_filename(&"日本語".repeat(100), "mp3", "ja");
        assert!(name.len() <= 204);
        assert!(name.ends_with(".mp3"));
        assert!(!name.contains('\u{fffd}'));
    }
}
