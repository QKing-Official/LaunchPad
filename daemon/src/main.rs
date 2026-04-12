// Imports

use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
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

// Main function, responsible for everything
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load the info from env and start the daemon with it
    dotenvy::dotenv().ok();

    // Trace the logs
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("daemon=debug".parse()?))
        .init();

    let cfg = config::loader::load();

    // Check the db and migrate it
    let db = db::connect(&cfg.database_url).await?;
    db::migrate(&db).await?;
    info!("Database connected and migrated");

    let docker = DockerClient::new();
    info!("Docker client ready");

    let state = Arc::new(AppState::new(docker, db));

    let cors = CorsLayer::new().allow_origin(Any).allow_headers(Any).allow_methods(Any);
    let app  = api::routes::router(state).layer(cors);
    // Actually start the webserver api. HTTP API for ease
    
    let addr = format!("0.0.0.0:{}", cfg.port);
    let listener = TcpListener::bind(&addr).await?;
    info!("Daemon listening on http://{}", addr);

    axum::serve(listener, app).await?;
    Ok(())
}