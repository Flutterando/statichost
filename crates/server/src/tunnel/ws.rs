use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    http::{header, HeaderMap, StatusCode},
    response::Response,
};
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use statichost_proto::{decode, encode, is_reserved, is_valid_name, TunnelMsg};
use tokio::sync::mpsc;

use crate::auth::ct_eq;
use crate::state::AppState;
use crate::tunnel::registry::{BodyEvent, ResponseHead, TunnelHandle};

#[derive(Deserialize)]
pub struct TunnelParams {
    pub name: String,
    pub token: Option<String>,
}

pub async fn upgrade(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(params): Query<TunnelParams>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string())
        .or(params.token);

    let token = token.ok_or(StatusCode::UNAUTHORIZED)?;
    if !ct_eq(token.as_bytes(), state.config.token.as_bytes()) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let name = params.name;
    if !is_valid_name(&name) || is_reserved(&name) || name == state.config.api_subdomain {
        return Err(StatusCode::BAD_REQUEST);
    }

    Ok(ws.on_upgrade(move |socket| handle_socket(socket, state, name)))
}

async fn handle_socket(socket: WebSocket, state: AppState, name: String) {
    let (mut ws_sink, mut ws_stream) = socket.split();
    let (out_tx, mut out_rx) = mpsc::channel::<TunnelMsg>(64);

    let handle = TunnelHandle::new(name.clone(), out_tx.clone());
    let handle_id = Arc::as_ptr(&handle) as usize;

    if let Err(e) = state.registry.register(name.clone(), handle.clone()).await {
        tracing::warn!("register {name}: {e}");
        let _ = ws_sink.send(Message::Close(None)).await;
        return;
    }
    tracing::info!("tunnel registered: {name}");

    let writer = tokio::spawn(async move {
        while let Some(msg) = out_rx.recv().await {
            match encode(&msg) {
                Ok(bytes) => {
                    if ws_sink.send(Message::Binary(bytes)).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    tracing::error!("encode: {e}");
                }
            }
        }
        let _ = ws_sink.send(Message::Close(None)).await;
    });

    let handle_for_reader = handle.clone();
    let reader = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_stream.next().await {
            match msg {
                Message::Binary(b) => match decode(&b) {
                    Ok(parsed) => route_inbound(&handle_for_reader, parsed).await,
                    Err(e) => tracing::warn!("decode: {e}"),
                },
                Message::Ping(p) => {
                    let _ = handle_for_reader
                        .send(TunnelMsg::Pong { nonce: nonce_from(&p) })
                        .await;
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    let pinger = {
        let h = handle.clone();
        tokio::spawn(async move {
            let mut nonce: u64 = 0;
            loop {
                tokio::time::sleep(Duration::from_secs(15)).await;
                nonce = nonce.wrapping_add(1);
                if h.send(TunnelMsg::Ping { nonce }).await.is_err() {
                    break;
                }
            }
        })
    };

    let _ = tokio::select! {
        _ = reader => (),
        _ = writer => (),
    };
    pinger.abort();

    state.registry.unregister(&name, handle_id).await;
    abort_pending(&handle);
    tracing::info!("tunnel closed: {name}");
}

fn nonce_from(b: &[u8]) -> u64 {
    let mut buf = [0u8; 8];
    let n = b.len().min(8);
    buf[..n].copy_from_slice(&b[..n]);
    u64::from_le_bytes(buf)
}

async fn route_inbound(handle: &TunnelHandle, msg: TunnelMsg) {
    match msg {
        TunnelMsg::ResponseStart {
            id,
            status,
            headers,
            has_body,
        } => {
            if let Some(mut entry) = handle.pending.get_mut(&id) {
                if let Some(tx) = entry.head_tx.take() {
                    let _ = tx.send(Ok(ResponseHead {
                        status,
                        headers,
                        has_body,
                    }));
                }
            }
        }
        TunnelMsg::BodyChunk { id, data, .. } => {
            if let Some(entry) = handle.pending.get(&id) {
                let _ = entry
                    .body_tx
                    .send(BodyEvent::Chunk(Bytes::from(data.into_vec())))
                    .await;
            }
        }
        TunnelMsg::BodyEnd { id } => {
            if let Some((_, entry)) = handle.pending.remove(&id) {
                let _ = entry.body_tx.send(BodyEvent::End).await;
            }
        }
        TunnelMsg::ResponseError { id, kind, msg } => {
            if let Some((_, mut entry)) = handle.pending.remove(&id) {
                if let Some(tx) = entry.head_tx.take() {
                    let _ = tx.send(Err((kind, msg.clone())));
                } else {
                    let _ = entry.body_tx.send(BodyEvent::Error(kind, msg)).await;
                }
            }
        }
        TunnelMsg::Pong { .. } | TunnelMsg::Ping { .. } => {}
        _ => {}
    }
}

fn abort_pending(handle: &TunnelHandle) {
    let ids: Vec<_> = handle.pending.iter().map(|e| *e.key()).collect();
    for id in ids {
        if let Some((_, mut entry)) = handle.pending.remove(&id) {
            if let Some(tx) = entry.head_tx.take() {
                let _ = tx.send(Err((
                    statichost_proto::ErrKind::BadGateway,
                    "tunnel closed".into(),
                )));
            }
        }
    }
}
