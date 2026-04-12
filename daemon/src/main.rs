// Imports

use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::{CorsLayer};
use axum::http::{HeaderValue, Method};
use tracing::info;
use tracing_subscriber::EnvFilter;

mod api;
mod config;
mod db;
mod docker;
mod filesystem;
mod monitoring;
mod server;
mod utils;

use crate::docker::client::DockerClient;
use crate::server::state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("daemon=debug".parse()?))
        .init();

    let cfg = config::loader::load();

    let db = db::connect(&cfg.database_url).await?;
    db::migrate(&db).await?;
    info!("Database connected and migrated");

    let docker = DockerClient::new();
    info!("Docker client ready");

    let state = Arc::new(AppState::new(docker, db));

    match db::queries::all_external_ports(&state.db).await {
        Ok(ports) => {
            for p in ports {
                state.ports.mark_used(p as u16);
            }
            info!("Port allocator seeded from database");
        }
        Err(e) => {
            tracing::warn!("Could not seed port allocator from DB: {}", e);
        }
    }

    // Restrict CORS
    let allowed_origin = std::env::var("ALLOWED_ORIGIN")
        .unwrap_or_else(|_| "http://localhost:3000".to_string());

    let cors = CorsLayer::new()
        .allow_origin(
            allowed_origin
                .parse::<HeaderValue>()
                .expect("ALLOWED_ORIGIN is not a valid header value"),
        )
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::HeaderName::from_static("x-api-key"),
        ])
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS]);

    let app = api::routes::router(state).layer(cors);

    // Bind to localhost only by default 
    // set BIND_ADDR=0.0.0.0:8000 to expose externally
    let bind_addr = std::env::var("BIND_ADDR")
        .unwrap_or_else(|_| format!("127.0.0.1:{}", cfg.port));
    let listener = TcpListener::bind(&bind_addr).await?;
    info!("Daemon listening on http://{}", bind_addr);

    axum::serve(listener, app).await?;
    Ok(())
}