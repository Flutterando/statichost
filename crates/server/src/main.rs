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
    Router,
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
    let state = AppState::new(cfg);

    let api_routes = routes::api::router()
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_bearer,
        ))
        .with_state(state.clone());

    let tunnel_route = Router::new()
        .route("/api/tunnel", get(tunnel::ws::upgrade))
        .with_state(state.clone());

    let downloads_route = routes::downloads::router().with_state(state.clone());

    let dispatch = Router::new()
        .fallback(any(router::dispatch))
        .with_state(state.clone());

    let app = Router::new()
        .nest("/api", api_routes)
        .merge(tunnel_route)
        .merge(downloads_route)
        .merge(dispatch)
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
