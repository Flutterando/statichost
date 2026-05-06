use std::path::{Path, PathBuf};

use axum::{
    extract::{multipart::Multipart, DefaultBodyLimit, Path as AxumPath, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use serde::Serialize;
use statichost_proto::{is_reserved, is_valid_name};

use crate::state::AppState;

const MAX_UPLOAD_BYTES: usize = 200 * 1024 * 1024;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/deploy", post(deploy))
        .route("/sites", get(list_sites))
        .route("/sites/:name", delete(delete_site))
        .route("/health", get(health))
        .layer(DefaultBodyLimit::max(MAX_UPLOAD_BYTES))
}

#[derive(Serialize)]
struct DeployResponse {
    name: String,
    url: String,
}

#[derive(Serialize)]
struct SiteInfo {
    name: String,
    files: u64,
    bytes: u64,
}

async fn health() -> &'static str {
    "ok"
}

async fn deploy(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut name: Option<String> = None;
    let mut archive: Option<Vec<u8>> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("multipart: {e}")))?
    {
        match field.name().unwrap_or("") {
            "name" => {
                name = Some(field.text().await.map_err(|e| {
                    (StatusCode::BAD_REQUEST, format!("name field: {e}"))
                })?);
            }
            "archive" => {
                let bytes = field.bytes().await.map_err(|e| {
                    (StatusCode::PAYLOAD_TOO_LARGE, format!("archive: {e}"))
                })?;
                archive = Some(bytes.to_vec());
            }
            _ => {}
        }
    }

    let name = name.ok_or((StatusCode::BAD_REQUEST, "missing 'name'".into()))?;
    let archive = archive.ok_or((StatusCode::BAD_REQUEST, "missing 'archive'".into()))?;

    if !is_valid_name(&name) {
        return Err((StatusCode::BAD_REQUEST, "invalid name".into()));
    }
    if is_reserved(&name) || name == state.config.api_subdomain {
        return Err((StatusCode::BAD_REQUEST, "reserved name".into()));
    }

    let target = state.config.sites_dir.join(&name);
    let staging = state
        .config
        .sites_dir
        .join(format!(".staging-{name}-{}", uuid::Uuid::new_v4()));

    extract_tar_gz(&archive, &staging)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("extract: {e}")))?;

    if target.exists() {
        let trash = state
            .config
            .sites_dir
            .join(format!(".trash-{name}-{}", uuid::Uuid::new_v4()));
        std::fs::rename(&target, &trash)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("trash: {e}")))?;
        let _ = std::fs::remove_dir_all(&trash);
    }

    std::fs::rename(&staging, &target).map_err(|e| {
        let _ = std::fs::remove_dir_all(&staging);
        (StatusCode::INTERNAL_SERVER_ERROR, format!("swap: {e}"))
    })?;

    let url = format!("https://{name}.{}", state.config.domain);
    tracing::info!("deployed {name} -> {url}");

    Ok((
        StatusCode::OK,
        Json(DeployResponse { name, url }),
    ))
}

async fn list_sites(
    State(state): State<AppState>,
) -> Result<Json<Vec<SiteInfo>>, (StatusCode, String)> {
    let mut out = Vec::new();
    let dir = match std::fs::read_dir(&state.config.sites_dir) {
        Ok(d) => d,
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("read_dir: {e}"))),
    };
    for entry in dir.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let (files, bytes) = dir_size(&entry.path());
        out.push(SiteInfo { name, files, bytes });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(Json(out))
}

async fn delete_site(
    State(state): State<AppState>,
    AxumPath(name): AxumPath<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    if !is_valid_name(&name) {
        return Err((StatusCode::BAD_REQUEST, "invalid name".into()));
    }
    let target = state.config.sites_dir.join(&name);
    if !target.is_dir() {
        return Err((StatusCode::NOT_FOUND, "no such site".into()));
    }
    std::fs::remove_dir_all(&target)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("remove: {e}")))?;
    tracing::info!("deleted {name}");
    Ok(StatusCode::NO_CONTENT)
}

fn extract_tar_gz(bytes: &[u8], dest: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(dest)?;
    let gz = flate2::read::GzDecoder::new(bytes);
    let mut ar = tar::Archive::new(gz);
    ar.set_preserve_permissions(false);
    for entry in ar.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        if path.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
            anyhow::bail!("archive contains '..'");
        }
        let out = dest.join(&path);
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)?;
        }
        entry.unpack(&out)?;
    }
    Ok(())
}

fn dir_size(p: &Path) -> (u64, u64) {
    let mut files = 0u64;
    let mut bytes = 0u64;
    for entry in walkdir::WalkDir::new(p).into_iter().flatten() {
        if entry.file_type().is_file() {
            files += 1;
            bytes += entry.metadata().map(|m| m.len()).unwrap_or(0);
        }
    }
    (files, bytes)
}

#[allow(dead_code)]
fn _ensure_path_use(_: &PathBuf) {}
