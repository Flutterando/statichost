use std::time::Duration;

use axum::{
    body::Body,
    extract::Request,
    http::{header, HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use futures_util::StreamExt;
use serde_bytes::ByteBuf;
use statichost_proto::{ErrKind, TunnelMsg, MAX_CHUNK_SIZE};
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

use crate::tunnel::registry::{BodyEvent, PendingRequest, TunnelHandle};

const HEAD_TIMEOUT: Duration = Duration::from_secs(30);
const BODY_IDLE_TIMEOUT: Duration = Duration::from_secs(60);

const HOP_BY_HOP: &[&str] = &[
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailers",
    "transfer-encoding",
    "upgrade",
];

pub async fn proxy_request(
    handle: std::sync::Arc<TunnelHandle>,
    subdomain: &str,
    base_domain: &str,
    req: Request,
) -> Response {
    let (parts, body) = req.into_parts();

    if parts
        .headers
        .get(header::UPGRADE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.eq_ignore_ascii_case("websocket"))
        .unwrap_or(false)
    {
        return (StatusCode::NOT_IMPLEMENTED, "WebSocket tunneling not supported in MVP")
            .into_response();
    }

    let id = Uuid::new_v4();
    let path = parts.uri.path().to_string();
    let query = parts.uri.query().map(|s| s.to_string());

    let client_ip = parts
        .headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_default();

    let mut headers = filter_headers(&parts.headers);
    headers.push(("x-forwarded-host".into(), format!("{subdomain}.{base_domain}")));
    headers.push(("x-forwarded-proto".into(), "https".into()));
    if !client_ip.is_empty() {
        headers.push(("x-forwarded-for".into(), client_ip));
    }

    let has_body = body_might_have_content(&parts.headers);

    let (head_tx, head_rx) = oneshot::channel();
    let (body_tx, body_rx) = mpsc::channel::<BodyEvent>(16);
    handle.pending.insert(
        id,
        PendingRequest {
            head_tx: Some(head_tx),
            body_tx,
        },
    );

    let send_start = handle
        .send(TunnelMsg::RequestStart {
            id,
            method: parts.method.as_str().to_string(),
            path,
            query,
            headers,
            has_body,
        })
        .await;

    if send_start.is_err() {
        handle.pending.remove(&id);
        return (StatusCode::BAD_GATEWAY, "tunnel closed").into_response();
    }

    if has_body {
        let h = handle.clone();
        tokio::spawn(async move {
            let mut stream = body.into_data_stream();
            let mut seq: u32 = 0;
            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        for slice in bytes.chunks(MAX_CHUNK_SIZE) {
                            if h.send(TunnelMsg::BodyChunk {
                                id,
                                seq,
                                data: ByteBuf::from(slice.to_vec()),
                            })
                            .await
                            .is_err()
                            {
                                return;
                            }
                            seq = seq.wrapping_add(1);
                        }
                    }
                    Err(_) => {
                        let _ = h.send(TunnelMsg::Cancel { id }).await;
                        return;
                    }
                }
            }
            let _ = h.send(TunnelMsg::BodyEnd { id }).await;
        });
    } else {
        let _ = handle.send(TunnelMsg::BodyEnd { id }).await;
    }

    let head = match tokio::time::timeout(HEAD_TIMEOUT, head_rx).await {
        Ok(Ok(Ok(h))) => h,
        Ok(Ok(Err((kind, msg)))) => {
            handle.pending.remove(&id);
            return error_response(kind, &msg);
        }
        Ok(Err(_)) => {
            handle.pending.remove(&id);
            return (StatusCode::BAD_GATEWAY, "tunnel dropped").into_response();
        }
        Err(_) => {
            handle.pending.remove(&id);
            let _ = handle.send(TunnelMsg::Cancel { id }).await;
            return (StatusCode::GATEWAY_TIMEOUT, "tunnel timeout").into_response();
        }
    };

    let mut resp = Response::builder().status(head.status);
    let resp_headers = resp.headers_mut().expect("builder");
    for (k, v) in &head.headers {
        if HOP_BY_HOP.iter().any(|h| h.eq_ignore_ascii_case(k)) {
            continue;
        }
        if let (Ok(name), Ok(val)) = (HeaderName::from_bytes(k.as_bytes()), HeaderValue::from_str(v)) {
            resp_headers.append(name, val);
        }
    }

    let body = if head.has_body {
        let stream = stream_body(body_rx);
        Body::from_stream(stream)
    } else {
        Body::empty()
    };

    resp.body(body).unwrap_or_else(|_| {
        (StatusCode::INTERNAL_SERVER_ERROR, "build response").into_response()
    })
}

fn stream_body(
    mut rx: mpsc::Receiver<BodyEvent>,
) -> impl futures_util::Stream<Item = Result<Bytes, std::io::Error>> + Send + 'static {
    async_stream::stream! {
        loop {
            match tokio::time::timeout(BODY_IDLE_TIMEOUT, rx.recv()).await {
                Ok(Some(BodyEvent::Chunk(b))) => yield Ok(b),
                Ok(Some(BodyEvent::End)) => break,
                Ok(Some(BodyEvent::Error(_, m))) => {
                    yield Err(std::io::Error::other(m));
                    break;
                }
                Ok(None) => break,
                Err(_) => {
                    yield Err(std::io::Error::other("tunnel body idle timeout"));
                    break;
                }
            }
        }
    }
}

fn filter_headers(h: &HeaderMap) -> Vec<(String, String)> {
    let mut out = Vec::with_capacity(h.len());
    for (name, value) in h.iter() {
        let n = name.as_str();
        if HOP_BY_HOP.iter().any(|hop| hop.eq_ignore_ascii_case(n)) {
            continue;
        }
        if n.eq_ignore_ascii_case("host") {
            continue;
        }
        if let Ok(v) = value.to_str() {
            out.push((n.to_string(), v.to_string()));
        }
    }
    out
}

fn body_might_have_content(h: &HeaderMap) -> bool {
    if let Some(cl) = h.get(header::CONTENT_LENGTH).and_then(|v| v.to_str().ok()) {
        if let Ok(n) = cl.parse::<u64>() {
            return n > 0;
        }
    }
    h.get(header::TRANSFER_ENCODING).is_some()
}

fn error_response(kind: ErrKind, msg: &str) -> Response {
    let status = match kind {
        ErrKind::LocalRefused | ErrKind::BadGateway => StatusCode::BAD_GATEWAY,
        ErrKind::LocalTimeout => StatusCode::GATEWAY_TIMEOUT,
        ErrKind::Internal => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (status, msg.to_string()).into_response()
}
