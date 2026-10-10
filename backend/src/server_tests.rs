use crate::{commands, controller, server, state::StudioState, storage::Store};
use anyhow::Result;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::path::PathBuf;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tower::ServiceExt;

#[tokio::test]
async fn media_ranges_read_directly_from_saved_archive_and_reject_invalid_ranges() -> Result<()> {
    let temporary = tempfile::tempdir()?;
    let mut store = Store::open(
        temporary.path().join("config"),
        Some(temporary.path().join("work")),
    )?;
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/studio-check.mp4");
    let original = std::fs::read(&fixture)?;
    commands::import_source(
        &mut store,
        fixture,
        "video.mp4".into(),
        "video".into(),
        None,
        0.,
    )?;
    let asset = store.project.assets[0].id.clone();
    store.save(&temporary.path().join("session.justspeak"))?;
    let root = store.root();
    let app = server::router(StudioState::new(store));
    for (range, expected) in [
        ("bytes=4-19", original[4..20].to_vec()),
        ("bytes=-8", original[original.len() - 8..].to_vec()),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/media/{asset}"))
                    .header("Range", range)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(response.headers()["content-type"], "video/mp4");
        assert_eq!(
            to_bytes(response.into_body(), 1024).await?.as_ref(),
            expected
        );
    }
    let invalid = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/media/{asset}"))
                .header("Range", "bytes=999999999-")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(invalid.status(), StatusCode::RANGE_NOT_SATISFIABLE);
    let index = app
        .oneshot(Request::builder().uri("/").body(Body::empty())?)
        .await?;
    assert_eq!(index.headers()["content-type"], "text/html");
    assert!(!root.exists());
    Ok(())
}

#[tokio::test]
#[cfg(not(target_os = "windows"))]
async fn video_preview_masks_audio_metadata_across_ranges_without_changing_saved_sources()
-> Result<()> {
    let temporary = tempfile::tempdir()?;
    let mut store = Store::open(
        temporary.path().join("config"),
        Some(temporary.path().join("work")),
    )?;
    let source =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/studio-check.mp4");
    let original = std::fs::read(&source)?;
    commands::import_source(
        &mut store,
        source,
        "video.mp4".into(),
        "video".into(),
        None,
        0.,
    )?;
    let asset = store.project.assets[0].id.clone();
    store.save(&temporary.path().join("session.justspeak"))?;
    let (path, base, length) = store.location(&asset, false)?;
    let patches = crate::video_source::patches(&path, base, length, "video.mp4")?;
    assert!(!patches.is_empty());
    let app = server::router(StudioState::new(store));
    for (position, replacement) in &patches {
        for offset in 0..replacement.len() {
            let start = position + offset as u64;
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(format!("/preview/{asset}"))
                        .header("Range", format!("bytes={start}-{start}"))
                        .body(Body::empty())?,
                )
                .await?;
            assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
            assert_eq!(
                to_bytes(response.into_body(), 1).await?.as_ref(),
                &replacement[offset..offset + 1]
            );
        }
    }
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/preview/{asset}"))
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(
        response.headers()["content-length"],
        original.len().to_string()
    );
    let preview = to_bytes(response.into_body(), original.len()).await?;
    let preview_path = temporary.path().join("preview.mp4");
    std::fs::write(&preview_path, &preview)?;
    let stream = symphonia::core::io::MediaSourceStream::new(
        Box::new(std::fs::File::open(preview_path)?),
        Default::default(),
    );
    let probe = symphonia::default::get_probe().format(
        &Default::default(),
        stream,
        &Default::default(),
        &Default::default(),
    )?;
    assert!(!probe.format.tracks().is_empty());
    assert!(
        probe
            .format
            .tracks()
            .iter()
            .all(|track| track.codec_params.sample_rate.is_none())
    );
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/media/{asset}"))
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(
        to_bytes(response.into_body(), original.len())
            .await?
            .as_ref(),
        original
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn websocket_is_audio_only_and_flush_fence_keeps_final_microphone_samples() -> Result<()> {
    let temporary = tempfile::tempdir()?;
    let store = Store::open(
        temporary.path().join("config"),
        Some(temporary.path().join("work")),
    )?;
    let state = StudioState::new(store);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let app = server::router(state.clone());
    let shutdown = state.shutdown.clone();
    let service = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async move { shutdown.notified().await })
            .await
    });
    let (mut socket, _) = connect_async(format!("ws://{address}/ws")).await?;
    assert!(connect_async(format!("ws://{address}/ws")).await.is_err());
    socket
        .send(Message::Text(
            json!({"command":"track_add"}).to_string().into(),
        ))
        .await?;
    socket.send(Message::Binary(vec![2].into())).await?;
    let ack: Value = serde_json::from_str(socket.next().await.unwrap()?.to_text()?)?;
    assert_eq!(ack["type"], "input-drained");
    assert_eq!(state.store.lock().unwrap().project.tracks.len(), 3);
    {
        let mut store = state.store.lock().unwrap();
        let mut audio = state.audio.lock().unwrap();
        controller::execute(
            &mut store,
            &mut audio,
            &json!({"command":"record_begin","position":1.25}),
        )?;
    }
    let mut mic = vec![1];
    for _ in 0..129 {
        mic.extend(0.5f32.to_le_bytes());
    }
    socket.send(Message::Binary(mic.into())).await?;
    socket.send(Message::Binary(vec![2].into())).await?;
    let input: Value = serde_json::from_str(socket.next().await.unwrap()?.to_text()?)?;
    assert_eq!(input["type"], "input");
    let fence: Value = serde_json::from_str(socket.next().await.unwrap()?.to_text()?)?;
    assert_eq!(fence["type"], "input-drained");
    {
        let mut store = state.store.lock().unwrap();
        let mut audio = state.audio.lock().unwrap();
        controller::execute(&mut store, &mut audio, &json!({"command":"record_end"}))?;
        assert_eq!(store.project.assets[0].frames, 129);
        assert_eq!(store.project.tracks[1].clips[0].start, 1.25);
    }
    socket.close(None).await?;
    let _ = socket.next().await;
    state.shutdown.notify_waiters();
    tokio::time::timeout(std::time::Duration::from_secs(3), service).await???;
    assert!(!state.connected.load(std::sync::atomic::Ordering::SeqCst));
    Ok(())
}

#[tokio::test]
async fn external_origin_cannot_request_app_close() -> Result<()> {
    let temporary = tempfile::tempdir()?;
    let state = StudioState::new(Store::open(
        temporary.path().join("config"),
        Some(temporary.path().join("work")),
    )?);
    let app = server::router(state.clone());
    let blocked = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/close")
                .header("Host", "127.0.0.1:5174")
                .header("Origin", "https://external.example")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(blocked.status(), StatusCode::FORBIDDEN);
    let valid = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/close")
                .header("Host", "127.0.0.1:5174")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(valid.status(), StatusCode::OK);
    tokio::time::timeout(
        std::time::Duration::from_millis(100),
        state.close_requested.notified(),
    )
    .await?;
    Ok(())
}
