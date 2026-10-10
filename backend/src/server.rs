use crate::state::StudioState;
use axum::{
    Router,
    body::Body,
    extract::{
        Path, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use futures_util::stream;
use rust_embed::RustEmbed;
use serde_json::json;
use std::{io::SeekFrom, path::PathBuf, sync::atomic::Ordering};
use tokio::io::{AsyncReadExt, AsyncSeekExt};

#[derive(RustEmbed)]
#[folder = "../dist/"]
struct Frontend;
pub fn router(state: StudioState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/close", post(request_close))
        .route("/ws", get(upgrade))
        .route("/media/{id}", get(media_file))
        .route("/preview/{id}", get(video_preview))
        .fallback(frontend)
        .with_state(state)
}
async fn health(State(state): State<StudioState>) -> impl IntoResponse {
    let store = state.store.try_lock();
    let details=store.ok().map(|store|json!({"workingDirectory":store.config.working_directory,"projectFile":store.config.project_file,"dirty":store.config.dirty}));
    axum::Json(
        json!({"app":"simpleVoiceover","version":env!("CARGO_PKG_VERSION"),"pid":std::process::id(),"busy":details.is_none(),"project":details}),
    )
}
fn same_origin(headers: &HeaderMap) -> bool {
    let host = headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let allowed_host = host.starts_with("127.0.0.1:") || host.starts_with("localhost:");
    allowed_host
        && headers.get(header::ORIGIN).is_none_or(|v| {
            v.to_str()
                .ok()
                .is_some_and(|origin| origin == format!("http://{host}"))
        })
}
async fn request_close(State(state): State<StudioState>, headers: HeaderMap) -> StatusCode {
    if !same_origin(&headers) {
        return StatusCode::FORBIDDEN;
    }
    state.close_requested.notify_one();
    StatusCode::OK
}
async fn upgrade(
    State(state): State<StudioState>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    if !same_origin(&headers) {
        return StatusCode::FORBIDDEN.into_response();
    }
    if state.connected.swap(true, Ordering::SeqCst) {
        return (
            StatusCode::CONFLICT,
            crate::i18n::message("error.alreadyOpen"),
        )
            .into_response();
    }
    ws.max_message_size(4 * 1024 * 1024 + 1)
        .max_frame_size(4 * 1024 * 1024 + 1)
        .on_upgrade(move |socket| connection(socket, state))
}
async fn frontend(uri: axum::http::Uri) -> Response {
    let name = uri.path().trim_start_matches('/');
    let name = if name.is_empty() { "index.html" } else { name };
    let asset = Frontend::get(name);
    match asset {
        Some(asset) => (
            [
                (
                    header::CONTENT_TYPE,
                    mime_guess::from_path(name)
                        .first_or_octet_stream()
                        .to_string(),
                ),
                (header::CACHE_CONTROL, "no-cache".into()),
            ],
            asset.data.into_owned(),
        )
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn media_file(
    State(state): State<StudioState>,
    Path(asset_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    media_response(state, asset_id, headers, false).await
}

async fn video_preview(
    State(state): State<StudioState>,
    Path(asset_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    media_response(state, asset_id, headers, true).await
}

async fn media_response(
    state: StudioState,
    asset_id: String,
    headers: HeaderMap,
    preview: bool,
) -> Response {
    let preview = preview && !cfg!(target_os = "windows");
    let location = {
        let store = state.store.lock().unwrap();
        store
            .project
            .assets
            .iter()
            .find(|a| a.id == asset_id)
            .map(|a| {
                store
                    .location(&asset_id, false)
                    .map(|location| (location, a.name.clone()))
            })
    };
    match location {
        Some(Ok(((path, base, length), name))) => {
            let patches = if preview {
                let source_path = path.clone();
                let source_name = name.clone();
                tokio::task::spawn_blocking(move || {
                    crate::video_source::patches(&source_path, base, length, &source_name)
                })
                .await
                .ok()
                .and_then(Result::ok)
                .unwrap_or_default()
            } else {
                Vec::new()
            };
            serve_region(path, base, length, &name, &headers, patches).await
        }
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}
async fn serve_region(
    path: PathBuf,
    base: u64,
    length: u64,
    name: &str,
    headers: &HeaderMap,
    patches: Vec<crate::video_source::Patch>,
) -> Response {
    let range = headers.get(header::RANGE).and_then(|v| v.to_str().ok());
    let (start, end, status) = if let Some(range) = range {
        let parsed = (|| {
            let r = range.strip_prefix("bytes=")?;
            let (from, to) = r.split_once('-')?;
            if from.is_empty() {
                let tail = to.parse::<u64>().ok()?;
                Some((length.saturating_sub(tail), length.checked_sub(1)?))
            } else {
                let start = from.parse::<u64>().ok()?;
                let end = if to.is_empty() {
                    length.checked_sub(1)?
                } else {
                    to.parse::<u64>().ok()?.min(length.checked_sub(1)?)
                };
                Some((start, end))
            }
        })();
        match parsed {
            Some((s, e)) if s <= e && e < length => (s, e, StatusCode::PARTIAL_CONTENT),
            _ => {
                return (
                    StatusCode::RANGE_NOT_SATISFIABLE,
                    [(header::CONTENT_RANGE, format!("bytes */{length}"))],
                )
                    .into_response();
            }
        }
    } else {
        (0, length.saturating_sub(1), StatusCode::OK)
    };
    let Ok(mut file) = tokio::fs::File::open(path).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if file.seek(SeekFrom::Start(base + start)).await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    let remaining = if length == 0 { 0 } else { end - start + 1 };
    let chunks = stream::try_unfold(
        (file, remaining, start, patches),
        |(mut file, remaining, position, patches)| async move {
            if remaining == 0 {
                return Ok::<_, std::io::Error>(None);
            }
            let mut bytes = vec![0u8; remaining.min(256 * 1024) as usize];
            let count = file.read(&mut bytes).await?;
            if count == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    crate::i18n::message("error.incompleteMedia"),
                ));
            }
            bytes.truncate(count);
            crate::video_source::apply(&patches, position, &mut bytes);
            Ok(Some((
                bytes,
                (
                    file,
                    remaining - count as u64,
                    position + count as u64,
                    patches,
                ),
            )))
        },
    );
    let mut response = Response::new(Body::from_stream(chunks));
    *response.status_mut() = status;
    let out = response.headers_mut();
    out.insert(
        header::CONTENT_TYPE,
        mime_guess::from_path(name)
            .first_or_octet_stream()
            .to_string()
            .parse()
            .unwrap(),
    );
    out.insert(
        header::CONTENT_LENGTH,
        remaining.to_string().parse().unwrap(),
    );
    out.insert(header::ACCEPT_RANGES, "bytes".parse().unwrap());
    if status == StatusCode::PARTIAL_CONTENT {
        out.insert(
            header::CONTENT_RANGE,
            format!("bytes {start}-{end}/{length}").parse().unwrap(),
        );
    }
    response
}

async fn connection(mut socket: WebSocket, state: StudioState) {
    let session = state.audio.clone();
    loop {
        let message =
            tokio::select! {message=socket.recv()=>message,_=state.shutdown.notified()=>None};
        let Some(Ok(message)) = message else { break };
        let reply = match message {
            Message::Binary(bytes) => {
                let connection = session.clone();
                let studio = state.store.clone();
                let result = tokio::task::spawn_blocking(move || {
                    if bytes.as_ref() == [3] {
                        let store = studio.lock().unwrap();
                        let (packet, meters) = connection.lock().unwrap().pull(&store)?;
                        Ok::<_, anyhow::Error>((Some(packet), meters))
                    } else {
                        Ok((None, connection.lock().unwrap().binary(&bytes)?))
                    }
                })
                .await;
                match result {
                    Ok(Ok((packet, value))) => {
                        if let Some(packet) = packet
                            && socket.send(Message::Binary(packet.into())).await.is_err()
                        {
                            break;
                        }
                        Message::Text(value.to_string().into())
                    }
                    Ok(Err(e)) => Message::Text(
                        json!({"type":"error","error":e.to_string()})
                            .to_string()
                            .into(),
                    ),
                    Err(_) => break,
                }
            }
            Message::Close(frame) => {
                let _ = socket.send(Message::Close(frame)).await;
                break;
            }
            Message::Ping(bytes) => Message::Pong(bytes),
            _ => continue,
        };
        if socket.send(reply).await.is_err() {
            break;
        }
    }
    let _ = tokio::task::spawn_blocking(move || {
        let mut store = state.store.lock().unwrap();
        session.lock().unwrap().finish_record(&mut store)
    })
    .await;
    state.connected.store(false, Ordering::SeqCst);
}
