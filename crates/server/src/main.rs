mod auth;
mod config;
mod proxy;
mod router;
mod routes;
mod state;
mod tunnel;

use std::net::SocketAddr;

use axum::{
    middleware,
    routing::{any, get},
    Extension, Router,
};
use tower_http::trace::TraceLayer;

use crate::{config::Config, state::AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,statichost_server=debug")),
        )
        .init();

    let cfg = Config::from_env()?;
    let port = cfg.port;
    let api_host = format!("{}.{}", cfg.api_subdomain, cfg.domain);
    let state = AppState::new(cfg);

    let api_with_auth = routes::api::router()
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_bearer,
        ));

    let system_router: Router = Router::new()
        .nest("/api", api_with_auth)
        .route("/api/tunnel", get(tunnel::ws::upgrade))
        .with_state(state.clone());

    let app = Router::new()
        .fallback(any(router::dispatch))
        .layer(Extension(system_router))
        .with_state(state.clone())
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("listening on {addr}");
    tracing::info!("API endpoints expected on https://{api_host}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
