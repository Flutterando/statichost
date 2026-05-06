use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use http_body_util::{BodyExt, Full, StreamBody};
use hyper::body::{Frame, Incoming};
use hyper::header::HeaderName;
use hyper::Request as HyperRequest;
use hyper_util::client::legacy::Client as HyperClient;
use hyper_util::rt::TokioExecutor;
use serde_bytes::ByteBuf;
use statichost_proto::{decode, encode, ErrKind, TunnelMsg, MAX_CHUNK_SIZE};
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::tungstenite::http::Request as WsHttpRequest;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::connect_async;
use uuid::Uuid;

use crate::config::load;

pub async fn run(port: u16, name: String) -> Result<()> {
    let cfg = load()?;
    let url = build_ws_url(&cfg.host, &name)?;

    let mut backoff = Duration::from_secs(1);
    loop {
        match connect_and_run(&url, &cfg.token, port, &name).await {
            Ok(()) => {
                println!("Tunnel closed cleanly. Reconnecting...");
                backoff = Duration::from_secs(1);
            }
            Err(e) => {
                eprintln!("Tunnel error: {e}");
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(Duration::from_secs(30));
            }
        }
    }
}

fn build_ws_url(host: &str, name: &str) -> Result<String> {
    let mut url = url::Url::parse(host).context("invalid host URL")?;
    let scheme = match url.scheme() {
        "http" => "ws",
        "https" => "wss",
        other => anyhow::bail!("unsupported scheme: {other}"),
    };
    url.set_scheme(scheme)
        .map_err(|_| anyhow::anyhow!("set scheme"))?;
    url.set_path("/api/tunnel");
    url.set_query(Some(&format!("name={name}")));
    Ok(url.to_string())
}

async fn connect_and_run(url: &str, token: &str, port: u16, name: &str) -> Result<()> {
    let req = WsHttpRequest::builder()
        .uri(url)
        .header("Authorization", format!("Bearer {token}"))
        .header("Host", url::Url::parse(url)?.host_str().unwrap_or(""))
        .header("Connection", "Upgrade")
        .header("Upgrade", "websocket")
        .header("Sec-WebSocket-Version", "13")
        .header(
            "Sec-WebSocket-Key",
            tokio_tungstenite::tungstenite::handshake::client::generate_key(),
        )
        .body(())?;

    let (ws, _resp) = connect_async(req).await.context("ws connect")?;
    println!("Tunnel '{name}' connected -> forwarding to localhost:{port}");

    let (sink, mut stream) = ws.split();
    let sink = Arc::new(Mutex::new(sink));

    let (out_tx, mut out_rx) = mpsc::channel::<TunnelMsg>(64);

    let writer_sink = sink.clone();
    let writer = tokio::spawn(async move {
        while let Some(msg) = out_rx.recv().await {
            let bytes = match encode(&msg) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("encode: {e}");
                    continue;
                }
            };
            let mut s = writer_sink.lock().await;
            if s.send(Message::Binary(bytes)).await.is_err() {
                break;
            }
        }
    });

    let body_channels: Arc<dashmap_like::Map> = Arc::new(dashmap_like::Map::default());

    while let Some(msg) = stream.next().await {
        let msg = msg.context("ws read")?;
        match msg {
            Message::Binary(b) => match decode(&b) {
                Ok(parsed) => handle_inbound(parsed, &out_tx, &body_channels, port).await,
                Err(e) => eprintln!("decode: {e}"),
            },
            Message::Ping(p) => {
                let mut s = sink.lock().await;
                let _ = s.send(Message::Pong(p)).await;
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
    writer.abort();
    Ok(())
}

mod dashmap_like {
    use std::sync::Mutex;
    use std::collections::HashMap;
    use uuid::Uuid;
    use bytes::Bytes;
    use tokio::sync::mpsc;

    #[derive(Default)]
    pub struct Map {
        inner: Mutex<HashMap<Uuid, mpsc::Sender<Option<Bytes>>>>,
    }

    impl Map {
        pub fn insert(&self, id: Uuid, tx: mpsc::Sender<Option<Bytes>>) {
            self.inner.lock().unwrap().insert(id, tx);
        }
        pub fn get(&self, id: &Uuid) -> Option<mpsc::Sender<Option<Bytes>>> {
            self.inner.lock().unwrap().get(id).cloned()
        }
        pub fn remove(&self, id: &Uuid) -> Option<mpsc::Sender<Option<Bytes>>> {
            self.inner.lock().unwrap().remove(id)
        }
    }
}

async fn handle_inbound(
    msg: TunnelMsg,
    out_tx: &mpsc::Sender<TunnelMsg>,
    bodies: &Arc<dashmap_like::Map>,
    port: u16,
) {
    match msg {
        TunnelMsg::RequestStart {
            id,
            method,
            path,
            query,
            headers,
            has_body,
        } => {
            let (body_tx, body_rx) = mpsc::channel::<Option<Bytes>>(16);
            if has_body {
                bodies.insert(id, body_tx);
            } else {
                drop(body_tx);
            }
            let out = out_tx.clone();
            let bodies_for_cleanup = bodies.clone();
            tokio::spawn(async move {
                let res =
                    perform_local_request(id, method, path, query, headers, has_body, body_rx, out.clone(), port)
                        .await;
                if let Err(e) = res {
                    let _ = out
                        .send(TunnelMsg::ResponseError {
                            id,
                            kind: e.0,
                            msg: e.1,
                        })
                        .await;
                }
                bodies_for_cleanup.remove(&id);
            });
        }
        TunnelMsg::BodyChunk { id, data, .. } => {
            if let Some(tx) = bodies.get(&id) {
                let _ = tx.send(Some(Bytes::from(data.into_vec()))).await;
            }
        }
        TunnelMsg::BodyEnd { id } => {
            if let Some(tx) = bodies.remove(&id) {
                let _ = tx.send(None).await;
            }
        }
        TunnelMsg::Cancel { id } => {
            bodies.remove(&id);
        }
        TunnelMsg::Ping { nonce } => {
            let _ = out_tx.send(TunnelMsg::Pong { nonce }).await;
        }
        _ => {}
    }
}

async fn perform_local_request(
    id: Uuid,
    method: String,
    path: String,
    query: Option<String>,
    headers: Vec<(String, String)>,
    has_body: bool,
    mut body_rx: mpsc::Receiver<Option<Bytes>>,
    out: mpsc::Sender<TunnelMsg>,
    port: u16,
) -> Result<(), (ErrKind, String)> {
    let target = match query {
        Some(q) => format!("http://127.0.0.1:{port}{path}?{q}"),
        None => format!("http://127.0.0.1:{port}{path}"),
    };

    let mut builder = HyperRequest::builder()
        .method(method.as_str())
        .uri(&target);
    {
        let h = builder
            .headers_mut()
            .ok_or((ErrKind::Internal, "build req".into()))?;
        for (k, v) in headers {
            if k.eq_ignore_ascii_case("host") || k.eq_ignore_ascii_case("content-length") {
                continue;
            }
            if let (Ok(name), Ok(val)) = (
                HeaderName::from_bytes(k.as_bytes()),
                hyper::header::HeaderValue::from_str(&v),
            ) {
                h.append(name, val);
            }
        }
        h.insert(
            hyper::header::HOST,
            hyper::header::HeaderValue::from_str(&format!("127.0.0.1:{port}"))
                .map_err(|_| (ErrKind::Internal, "host header".into()))?,
        );
    }

    let client: HyperClient<_, BoxBody> =
        HyperClient::builder(TokioExecutor::new()).build_http();

    let req = if has_body {
        let stream = async_stream::stream! {
            while let Some(chunk) = body_rx.recv().await {
                match chunk {
                    Some(b) => yield Ok::<_, std::io::Error>(Frame::data(b)),
                    None => return,
                }
            }
        };
        let body: BoxBody = BodyExt::boxed(StreamBody::new(stream));
        builder.body(body).map_err(|e| (ErrKind::Internal, e.to_string()))?
    } else {
        let body: BoxBody = BodyExt::boxed(
            Full::new(Bytes::new())
                .map_err(|e: std::convert::Infallible| match e {}),
        );
        builder.body(body).map_err(|e| (ErrKind::Internal, e.to_string()))?
    };

    let resp = match client.request(req).await {
        Ok(r) => r,
        Err(e) => {
            let kind = if e.is_connect() {
                ErrKind::LocalRefused
            } else {
                ErrKind::Internal
            };
            return Err((kind, e.to_string()));
        }
    };

    let status = resp.status().as_u16();
    let mut hdrs = Vec::with_capacity(resp.headers().len());
    for (k, v) in resp.headers() {
        if let Ok(s) = v.to_str() {
            hdrs.push((k.as_str().to_string(), s.to_string()));
        }
    }

    let mut body = resp.into_body();
    let mut has_first_chunk = false;
    let mut buffered_first: Option<Bytes> = None;
    if let Some(frame) = body.frame().await {
        match frame {
            Ok(f) => {
                if let Ok(data) = f.into_data() {
                    if !data.is_empty() {
                        has_first_chunk = true;
                        buffered_first = Some(data);
                    }
                }
            }
            Err(e) => return Err((ErrKind::Internal, e.to_string())),
        }
    }

    out.send(TunnelMsg::ResponseStart {
        id,
        status,
        headers: hdrs,
        has_body: has_first_chunk || body_might_have_more(&body),
    })
    .await
    .map_err(|_| (ErrKind::Internal, "out closed".into()))?;

    let mut seq: u32 = 0;
    if let Some(bytes) = buffered_first {
        send_chunks(&out, id, &mut seq, bytes).await?;
    }

    while let Some(frame) = body.frame().await {
        match frame {
            Ok(f) => {
                if let Ok(data) = f.into_data() {
                    if !data.is_empty() {
                        send_chunks(&out, id, &mut seq, data).await?;
                    }
                }
            }
            Err(e) => return Err((ErrKind::Internal, e.to_string())),
        }
    }

    out.send(TunnelMsg::BodyEnd { id })
        .await
        .map_err(|_| (ErrKind::Internal, "out closed".into()))?;
    Ok(())
}

type BoxBody = http_body_util::combinators::BoxBody<Bytes, std::io::Error>;

fn body_might_have_more(_b: &Incoming) -> bool {
    true
}

async fn send_chunks(
    out: &mpsc::Sender<TunnelMsg>,
    id: Uuid,
    seq: &mut u32,
    data: Bytes,
) -> Result<(), (ErrKind, String)> {
    for slice in data.chunks(MAX_CHUNK_SIZE) {
        out.send(TunnelMsg::BodyChunk {
            id,
            seq: *seq,
            data: ByteBuf::from(slice.to_vec()),
        })
        .await
        .map_err(|_| (ErrKind::Internal, "out closed".into()))?;
        *seq = seq.wrapping_add(1);
    }
    Ok(())
}
