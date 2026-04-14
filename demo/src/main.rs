// Imports

use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::{CorsLayer};
use axum::http::{HeaderValue, Method};
use tracing::info;
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

mod api;
mod config;
mod db;
mod docker;
mod filesystem;
mod monitoring;
mod server;
mod utils;

use crate::docker::client::DockerClient;
use crate::docker::client::ContainerConfig;
use crate::server::state::{AppRecord, PortMappingRecord};
use crate::server::state::AppState;

async fn ensure_demo_app(state: &Arc<AppState>) {
    let existing = match db::queries::list_apps(&state.db).await {
        Ok(apps) => apps,
        Err(e) => {
            tracing::warn!("Could not list apps for demo precreate: {}", e);
            return;
        }
    };

    if let Some(first) = existing.first() {
        info!("Demo app already present: {} ({})", first.name, first.id);
        return;
    }

    let app_id = Uuid::new_v4();
    let app_name = "demo-app".to_string();
    let image = "alpine:3.20".to_string();
    let internal_port = 8080u16;
    let external_port = state.ports.allocate();

    let app = AppRecord {
        id: app_id,
        name: app_name.clone(),
        image: image.clone(),
        status: "pending".to_string(),
        container_id: None,
        memory_mb: None,
        cpu_shares: None,
        cpu_quota: None,
    };

    if let Err(e) = db::queries::insert_app(&state.db, &app).await {
        tracing::warn!("Could not insert demo app record: {}", e);
        return;
    }

    let mapping = PortMappingRecord {
        id: Uuid::new_v4(),
        app_id,
        internal_port: internal_port as i32,
        external_port: external_port as i32,
    };
    if let Err(e) = db::queries::insert_port_mapping(&state.db, &mapping).await {
        tracing::warn!("Could not insert demo app port mapping: {}", e);
        return;
    }

    let cfg = ContainerConfig {
        name: app_name.clone(),
        image: image.clone(),
        port_bindings: vec![(internal_port, external_port)],
        env: None,
        cmd: Some(vec!["sh".to_string(), "-c".to_string(), "sleep infinity".to_string()]),
        volumes: None,
        memory_mb: None,
        cpu_shares: None,
        network: None,
    };

    if let Err(e) = state.docker.pull_image(&image).await {
        let _ = db::queries::update_app_status(&state.db, app_id, "error", None).await;
        tracing::warn!("Could not pull demo app image: {}", e);
        return;
    }

    match state.docker.create_container(cfg).await {
        Ok(container_id) => {
            if let Err(e) = state.docker.start_container(&container_id).await {
                let _ = db::queries::update_app_status(&state.db, app_id, "error", Some(&container_id)).await;
                tracing::warn!("Could not start demo app container: {}", e);
                return;
            }
            let _ = db::queries::update_app_status(&state.db, app_id, "running", Some(&container_id)).await;
            info!(
                "Precreated demo app: {} ({}) on external port {}",
                app_name,
                app_id,
                external_port
            );
        }
        Err(e) => {
            let _ = db::queries::update_app_status(&state.db, app_id, "error", None).await;
            tracing::warn!("Could not create demo app container: {}", e);
        }
    }
}

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

    ensure_demo_app(&state).await;

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