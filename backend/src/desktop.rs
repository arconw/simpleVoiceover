use crate::{commands, controller, state::StudioState};
use anyhow::Result;
use serde_json::Value;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tauri::{Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder, WindowEvent};

struct DesktopState {
    studio: StudioState,
    allow_close: AtomicBool,
}

#[tauri::command]
async fn studio_command(
    app: tauri::AppHandle,
    state: State<'_, Arc<DesktopState>>,
    request: Value,
) -> std::result::Result<Value, String> {
    let studio = state.studio.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let mut store = studio
            .store
            .lock()
            .map_err(|_| anyhow::anyhow!("Хранилище недоступно"))?;
        let mut audio = studio
            .audio
            .lock()
            .map_err(|_| anyhow::anyhow!("Аудиодвижок недоступен"))?;
        let result = controller::execute(&mut store, &mut audio, &request);
        Ok::<_, anyhow::Error>((result, commands::snapshot(&store)))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    let (result, snapshot) = result;
    let _ = app.emit("studio-snapshot", snapshot);
    result.map_err(|e| e.to_string())
}

#[tauri::command]
fn close_studio(
    app: tauri::AppHandle,
    state: State<'_, Arc<DesktopState>>,
    without_saving: bool,
) -> std::result::Result<(), String> {
    let mut store = state
        .studio
        .store
        .try_lock()
        .map_err(|_| "Дождись завершения операции")?;
    let audio = state
        .studio
        .audio
        .try_lock()
        .map_err(|_| "Дождись завершения операции")?;
    crate::lifecycle::prepare_close(&mut store, &audio, without_saving)
        .map_err(|error| error.to_string())?;
    state.allow_close.store(true, Ordering::SeqCst);
    drop(audio);
    drop(store);
    if let Some(window) = app.get_webview_window("main") {
        window.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn run(studio: StudioState, address: &str) -> Result<()> {
    let desktop = Arc::new(DesktopState {
        studio: studio.clone(),
        allow_close: AtomicBool::new(false),
    });
    let url: tauri::Url = format!("http://{address}").parse()?;
    let closed = desktop.clone();
    tauri::Builder::default()
        .manage(desktop)
        .invoke_handler(tauri::generate_handler![studio_command, close_studio])
        .setup(move |app| {
            app.add_capability(
                tauri::ipc::CapabilityBuilder::new("studio")
                    .window("main")
                    .remote(format!("{}/*", url.origin().ascii_serialization()))
                    .permission("core:event:default")
                    .permission("allow-studio-command")
                    .permission("allow-close-studio"),
            )?;
            WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url.clone()))
                .title("simpleVoiceover")
                .inner_size(1440., 900.)
                .min_inner_size(960., 700.)
                .on_navigation(move |next| next.origin() == url.origin())
                .build()?;
            let handle = app.handle().clone();
            let close = studio.close_requested.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    close.notified().await;
                    let _ = handle.emit("studio-close-requested", ());
                }
            });
            Ok(())
        })
        .on_window_event(move |window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if !closed.allow_close.load(Ordering::SeqCst) {
                    api.prevent_close();
                    let _ = window.emit("studio-close-requested", ());
                }
            }
        })
        .run(tauri::generate_context!())?;
    Ok(())
}
