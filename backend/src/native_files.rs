use anyhow::Result;
#[cfg(not(windows))]
use anyhow::bail;
use std::path::PathBuf;

pub fn directory() -> Result<Option<PathBuf>> {
    #[cfg(windows)]
    {
        Ok(rfd::FileDialog::new()
            .set_title("Рабочий каталог simpleVoiceover")
            .pick_folder())
    }
    #[cfg(not(windows))]
    {
        bail!("Выбор папки доступен в Windows-приложении")
    }
}

pub fn project(save: bool) -> Result<Option<PathBuf>> {
    #[cfg(windows)]
    {
        let dialog = rfd::FileDialog::new()
            .add_filter("simpleVoiceover", &["justspeak"])
            .set_file_name("Моя сессия.justspeak");
        Ok(if save {
            dialog.save_file()
        } else {
            dialog.pick_file()
        })
    }
    #[cfg(not(windows))]
    {
        let _ = save;
        bail!("Выбор файла доступен в Windows-приложении")
    }
}

pub fn media() -> Result<Option<Vec<PathBuf>>> {
    #[cfg(windows)]
    {
        Ok(rfd::FileDialog::new()
            .set_title("Добавить медиа")
            .add_filter(
                "Аудио и видео",
                &[
                    "mp4", "mov", "m4a", "wav", "mp3", "flac", "ogg", "mkv", "webm",
                ],
            )
            .pick_files())
    }
    #[cfg(not(windows))]
    {
        bail!("Выбор файлов доступен в Windows-приложении")
    }
}

pub fn export(format: &str) -> Result<Option<PathBuf>> {
    #[cfg(windows)]
    {
        Ok(rfd::FileDialog::new()
            .set_title("Экспорт аудио")
            .add_filter(format.to_uppercase(), &[format])
            .set_file_name(format!("simpleVoiceover.{format}"))
            .save_file())
    }
    #[cfg(not(windows))]
    {
        let _ = format;
        bail!("Выбор файла доступен в Windows-приложении")
    }
}
