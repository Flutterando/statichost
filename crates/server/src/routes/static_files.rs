use std::path::{Path, PathBuf};

use axum::{
    body::Body,
    extract::Request,
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use tokio::fs::File;
use tokio_util::io::ReaderStream;

pub async fn serve(site_dir: &Path, req: Request) -> Response {
    let path = req.uri().path();
    let rel = path.trim_start_matches('/');

    let candidate = match safe_join(site_dir, rel) {
        Some(p) => p,
        None => return (StatusCode::FORBIDDEN, "bad path").into_response(),
    };

    if let Some(resp) = try_serve(&candidate).await {
        return resp;
    }

    if candidate.is_dir() {
        let idx = candidate.join("index.html");
        if let Some(resp) = try_serve(&idx).await {
            return resp;
        }
    }

    let fallback = site_dir.join("index.html");
    if let Some(resp) = try_serve(&fallback).await {
        return resp;
    }

    (StatusCode::NOT_FOUND, "not found").into_response()
}

async fn try_serve(path: &Path) -> Option<Response> {
    let meta = tokio::fs::metadata(path).await.ok()?;
    if !meta.is_file() {
        return None;
    }
    let file = File::open(path).await.ok()?;
    let len = meta.len();
    let mime = mime_for(path);
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let mut resp = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_LENGTH, len)
        .header(header::CONTENT_TYPE, HeaderValue::from_static(mime));

    if mime.starts_with("text/html") {
        resp = resp.header(header::CACHE_CONTROL, "no-cache");
    } else {
        resp = resp.header(header::CACHE_CONTROL, "public, max-age=300");
    }

    Some(resp.body(body).unwrap())
}

fn safe_join(base: &Path, rel: &str) -> Option<PathBuf> {
    let mut out = base.to_path_buf();
    for seg in rel.split('/') {
        if seg.is_empty() || seg == "." {
            continue;
        }
        if seg == ".." {
            return None;
        }
        out.push(seg);
    }
    Some(out)
}

fn mime_for(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "application/javascript; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "wasm" => "application/wasm",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "txt" => "text/plain; charset=utf-8",
        "xml" => "application/xml",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}
