use crate::{runtime, server, state, storage};
use anyhow::Result;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

pub async fn run() -> Result<()> {
    let mut port = 5174u16;
    let mut config = None;
    let mut project = None;
    let mut headless = false;
    let mut args = std::env::args().skip(1);
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--port" => port = args.next().unwrap_or_default().parse()?,
            "--config-dir" => config = Some(PathBuf::from(args.next().unwrap_or_default())),
            "--project" => project = Some(PathBuf::from(args.next().unwrap_or_default())),
            "--headless" => headless = true,
            _ => anyhow::bail!("Unknown argument: {argument}"),
        }
    }
    let directory = config.unwrap_or_else(|| {
        default_config_directory(
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir),
            std::env::var_os("LOCALAPPDATA").map(PathBuf::from),
        )
    });
    std::fs::create_dir_all(&directory)?;
    let log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(directory.join("simpleVoiceover.log"))?;
    tracing_subscriber::fmt()
        .with_target(false)
        .with_ansi(false)
        .with_writer(runtime::LogWriter(Arc::new(Mutex::new(log))))
        .init();
    let mut store = storage::Store::open(directory, None)?;
    if let Some(project) = project {
        store.load(&project)?;
    }
    let studio = state::StudioState::new(store);
    let address = format!("127.0.0.1:{port}");
    let listener = tokio::net::TcpListener::bind(&address).await?;
    let server_state = studio.clone();
    let shutdown = studio.shutdown.clone();
    let service = tokio::spawn(async move {
        axum::serve(listener, server::router(server_state))
            .with_graceful_shutdown(async move {
                shutdown.notified().await;
            })
            .await
    });
    tracing::info!("simpleVoiceover started: {address}");
    #[cfg(feature = "desktop")]
    if !headless {
        crate::desktop::run(studio.clone(), &address)?;
        studio.shutdown.notify_waiters();
        let _ = tokio::time::timeout(std::time::Duration::from_secs(5), service).await;
        return Ok(());
    }
    let _ = headless;
    tokio::select! {_=tokio::signal::ctrl_c()=>{},_=studio.close_requested.notified()=>{}}
    studio.shutdown.notify_waiters();
    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), service).await;
    Ok(())
}

#[cfg(target_os = "macos")]
fn default_config_directory(home: PathBuf, _local_app_data: Option<PathBuf>) -> PathBuf {
    home.join(".simpleVoiceover")
}

#[cfg(not(target_os = "macos"))]
fn default_config_directory(home: PathBuf, local_app_data: Option<PathBuf>) -> PathBuf {
    local_app_data
        .unwrap_or_else(|| home.join(".local/share"))
        .join("simpleVoiceover")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_keeps_settings_in_a_hidden_home_folder() {
        assert_eq!(
            default_config_directory(PathBuf::from("/Users/ada"), None),
            PathBuf::from("/Users/ada/.simpleVoiceover")
        );
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn other_platforms_prefer_local_app_data_then_xdg_share() {
        assert_eq!(
            default_config_directory(
                PathBuf::from("/home/ada"),
                Some(PathBuf::from("C:/Users/ada/AppData/Local"))
            ),
            PathBuf::from("C:/Users/ada/AppData/Local/simpleVoiceover")
        );
        assert_eq!(
            default_config_directory(PathBuf::from("/home/ada"), None),
            PathBuf::from("/home/ada/.local/share/simpleVoiceover")
        );
    }
}
