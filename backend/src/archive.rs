use crate::{filesystem::replace_file, model::Project};
use anyhow::{Result, ensure};
use std::{
    fs::{self, File},
    io::{self, Read},
    path::Path,
};
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};
pub fn read_manifest(path: &Path) -> Result<Project> {
    let mut archive = ZipArchive::new(File::open(path)?)?;
    let project: Project =
        serde_json::from_reader(archive.by_name("project.json")?.take(32 * 1024 * 1024))?;
    project.validate()?;
    Ok(project)
}

pub fn verify_archive(path: &Path, project: &Project) -> Result<()> {
    let mut archive = ZipArchive::new(File::open(path)?)?;
    ensure!(
        archive.len() == project.assets.len() * 2 + 1,
        "В проекте есть неожиданные файлы"
    );
    for asset in &project.assets {
        for (name, expected) in [
            (format!("media/{}", asset.id), asset.size),
            (format!("cache/{}.pcm", asset.id), asset.frames * 8),
        ] {
            let entry = archive.by_name(&name)?;
            ensure!(
                entry.size() == expected && entry.compression() == CompressionMethod::Stored,
                "Повреждённое медиа в проекте"
            );
        }
    }
    Ok(())
}

pub fn extract_archive(path: &Path, project: &Project, root: &Path) -> Result<()> {
    verify_archive(path, project)?;
    let mut archive = ZipArchive::new(File::open(path)?)?;
    for directory in ["media", "cache"] {
        fs::create_dir_all(root.join(directory))?;
    }
    for asset in &project.assets {
        for name in [
            format!("media/{}", asset.id),
            format!("cache/{}.pcm", asset.id),
        ] {
            let mut entry = archive.by_name(&name)?;
            let target = root.join(&name);
            ensure!(
                !target.exists(),
                "Найден незавершённый кэш. Исходный файл проекта сохранён."
            );
            let mut output = File::create(target)?;
            io::copy(&mut entry, &mut output)?;
            output.sync_all()?;
        }
    }
    Ok(())
}

pub fn save_archive(project: &Project, root: &Path, destination: &Path) -> Result<()> {
    project.validate()?;
    let temporary = destination.with_extension(format!("{}.partial", crate::model::id()));
    let mut zip = ZipWriter::new(File::create(&temporary)?);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .large_file(true);
    zip.start_file("project.json", options)?;
    serde_json::to_writer(&mut zip, project)?;
    for asset in &project.assets {
        for name in [
            format!("media/{}", asset.id),
            format!("cache/{}.pcm", asset.id),
        ] {
            zip.start_file(&name, options)?;
            io::copy(&mut File::open(root.join(&name))?, &mut zip)?;
        }
    }
    zip.finish()?.sync_all()?;
    replace_file(&temporary, destination)?;
    Ok(())
}
