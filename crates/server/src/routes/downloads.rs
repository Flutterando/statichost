use axum::{
    extract::{Path as AxumPath, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::state::AppState;

const INSTALL_SH: &str = include_str!("../../../../scripts/install.sh");
const INSTALL_PS1: &str = include_str!("../../../../scripts/install.ps1");

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/dl/:filename", get(dl))
        .route("/install.sh", get(install_sh))
        .route("/install.ps1", get(install_ps1))
}

async fn dl(
    State(state): State<AppState>,
    AxumPath(filename): AxumPath<String>,
) -> Result<Response, StatusCode> {
    if !filename
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        || filename.starts_with('.')
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    let path = state.config.binaries_dir.join(&filename);
    let meta = tokio::fs::metadata(&path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    if !meta.is_file() {
        return Err(StatusCode::NOT_FOUND);
    }
    let file = File::open(&path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let stream = ReaderStream::new(file);
    let body = axum::body::Body::from_stream(stream);
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_LENGTH, meta.len())
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{filename}\""),
        )
        .body(body)
        .unwrap())
}

async fn install_sh(State(state): State<AppState>) -> impl IntoResponse {
    let body = render(INSTALL_SH, &state);
    (
        [(header::CONTENT_TYPE, "text/x-shellscript; charset=utf-8")],
        body,
    )
}

async fn install_ps1(State(state): State<AppState>) -> impl IntoResponse {
    let body = render(INSTALL_PS1, &state);
    (
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        body,
    )
}

fn render(template: &str, state: &AppState) -> String {
    let host = format!(
        "https://{}.{}",
        state.config.api_subdomain, state.config.domain
    );
    template.replace("{{HOST}}", &host)
}
