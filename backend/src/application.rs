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
            _ => anyhow::bail!("Неизвестный аргумент: {argument}"),
        }
    }
    let directory = config.unwrap_or_else(|| {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(std::env::temp_dir)
                    .join(".local/share")
            })
            .join("simpleVoiceover")
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
    tracing::info!("simpleVoiceover запущен: {address}");
    #[cfg(all(windows, feature = "desktop"))]
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
