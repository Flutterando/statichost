use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Extension, Router,
};
use tower::ServiceExt;

use crate::{proxy, routes::static_files, state::AppState};

pub async fn dispatch(
    State(state): State<AppState>,
    Extension(system): Extension<Router>,
    req: Request,
) -> Response {
    let host: String = req
        .headers()
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .map(|s| strip_port(s).to_string())
        .unwrap_or_default();

    let base = state.config.domain.clone();
    let api_sub = state.config.api_subdomain.clone();

    let path = req.uri().path().to_string();
    let subdomain = match extract_subdomain(&host, &base) {
        Some(s) => s.to_string(),
        None => {
            return landing_or_404(&host, &base, &api_sub, &path);
        }
    };

    if subdomain == api_sub {
        return match system.oneshot(req).await {
            Ok(resp) => resp,
            Err(e) => match e {},
        };
    }

    if let Some(handle) = state.registry.get(&subdomain).await {
        return proxy::proxy_request(handle, &subdomain, &base, req).await;
    }

    let site_dir = state.config.sites_dir.join(&subdomain);
    if site_dir.is_dir() {
        return static_files::serve(&site_dir, req).await;
    }

    (StatusCode::NOT_FOUND, format!("no site or tunnel for '{subdomain}'")).into_response()
}

fn strip_port(host: &str) -> &str {
    match host.rfind(':') {
        Some(i) => &host[..i],
        None => host,
    }
}

fn extract_subdomain<'a>(host: &'a str, base: &str) -> Option<&'a str> {
    let host = host.trim_end_matches('.');
    if host.eq_ignore_ascii_case(base) {
        return None;
    }
    let suffix = format!(".{base}");
    if host.len() <= suffix.len() {
        return None;
    }
    let sub = &host[..host.len() - suffix.len()];
    if !host.to_lowercase().ends_with(&suffix.to_lowercase()) {
        return None;
    }
    if sub.is_empty() {
        None
    } else {
        Some(sub)
    }
}

fn landing_or_404(host: &str, base: &str, api_sub: &str, path: &str) -> Response {
    if host.eq_ignore_ascii_case(base) || host.is_empty() {
        if path == "/" {
            let body = format!(
                "statichost is running.\nAPI is on https://{api_sub}.{base}\n",
            );
            return (StatusCode::OK, body).into_response();
        }
        return (
            StatusCode::NOT_FOUND,
            format!("not found on {base}; API is on https://{api_sub}.{base}\n"),
        )
            .into_response();
    }
    (StatusCode::NOT_FOUND, format!("unknown host: {host}")).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_basic() {
        assert_eq!(extract_subdomain("foo.example.com", "example.com"), Some("foo"));
        assert_eq!(extract_subdomain("foo-bar.example.com", "example.com"), Some("foo-bar"));
        assert_eq!(extract_subdomain("example.com", "example.com"), None);
        assert_eq!(extract_subdomain("other.com", "example.com"), None);
    }

    #[test]
    fn strip_port_works() {
        assert_eq!(strip_port("foo.com:3000"), "foo.com");
        assert_eq!(strip_port("foo.com"), "foo.com");
    }
}
