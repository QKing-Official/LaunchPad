// Imports
// This app, in this file we have methods for mananging the apps inside our daemon. This includes creating, listing, getting info about, and deleting apps. Each app corresponds to a Docker container that we manage.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::db::queries;
use crate::docker::client::ContainerConfig;
use crate::server::state::{AppRecord, AppResponse, AppState, PortMappingRecord};

#[derive(Debug, Deserialize)]
pub struct CreateAppRequest {
    pub name:          String,
    pub image:         Option<String>,
    pub internal_port: Option<u16>,
    pub external_port: Option<u16>,
    pub env:           Option<Vec<String>>,
    pub cmd:           Option<Vec<String>>,
    pub volumes:       Option<Vec<String>>,
    pub memory_mb:     Option<i64>,
    pub cpu_shares:    Option<i64>,
}

// Response formatting
async fn build_response(pool: &sqlx::PgPool, app: &AppRecord) -> AppResponse {
    let ports: Vec<PortMappingRecord> = queries::get_port_mappings(pool, app.id)
        .await.unwrap_or_default();
    let first = ports.first();
    AppResponse {
        id:            app.id,
        name:          app.name.clone(),
        image:         app.image.clone(),
        status:        app.status.clone(),
        container_id:  app.container_id.clone(),
        external_port: first.map(|p| p.external_port),
        internal_port: first.map(|p| p.internal_port),
        memory_mb:     app.memory_mb,
        cpu_shares:    app.cpu_shares,
    }
}

async fn fire(db: &sqlx::PgPool, app_id: Uuid, status: &str, app_name: &str) {
    let hooks = match queries::list_webhooks(db, app_id).await {
        Ok(h)  => h,
        Err(_) => return,
    };
    let payload = serde_json::json!({
        "app_id": app_id, "app_name": app_name,
        "status": status, "ts": chrono::Utc::now().to_rfc3339(),
    });
    for hook in hooks {
        let url  = hook.url.clone();
        let body = payload.clone();
        tokio::spawn(async move {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build().unwrap_or_default();
            match client.post(&url).json(&body).send().await {
                Ok(r)  => tracing::info!("Webhook {} -> {}", url, r.status()),
                Err(e) => tracing::warn!("Webhook {} failed: {}", url, e),
            }
        });
    }
}


// App creation
pub async fn create_app(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateAppRequest>,
) -> impl IntoResponse {
    let id            = Uuid::new_v4();
    let image         = req.image.clone().unwrap_or_else(|| "python:3.12-slim".to_string());
    let internal_port = req.internal_port.unwrap_or(8000);
    let external_port = match req.external_port {
        Some(p) => { state.ports.mark_used(p); p }
        None    => state.ports.allocate(),
    };

    let vol_names     = req.volumes.clone().unwrap_or_default();
    let host_volumes: Vec<String> = vol_names.iter()
        .map(|v| format!("/srv/Launchpad/{}/volumes/{}", req.name, v))
        .collect();
    for dir in &host_volumes {
        if let Err(e) = std::fs::create_dir_all(dir) {
            return (StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("volume dir {}: {}", dir, e)}))).into_response();
        }
    }

    let app = AppRecord {
        id, name: req.name.clone(), image: image.clone(),
        status: "pending".to_string(), container_id: None,
        memory_mb: req.memory_mb.map(|v| v as i32),
        cpu_shares: req.cpu_shares.map(|v| v as i32),
        cpu_quota: None,
    };
    if let Err(e) = queries::insert_app(&state.db, &app).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response();
    }
    let _ = queries::insert_port_mapping(&state.db, &PortMappingRecord {
        id: Uuid::new_v4(), app_id: id,
        internal_port: internal_port as i32,
        external_port: external_port as i32,
    }).await;

    let net_name   = format!("launchpad_{}", req.name);
    let docker_net = state.docker.clone();
    let net_clone  = net_name.clone();
    tokio::spawn(async move { let _ = docker_net.ensure_network(&net_clone).await; });

    let docker   = state.docker.clone();
    let db       = state.db.clone();
    let name     = req.name.clone();
    let image_bg = image.clone();
    let cfg = ContainerConfig {
        name:          req.name.clone(),
        image:         image.clone(),
        port_bindings: vec![(internal_port, external_port)],
        env:           req.env.clone(),
        cmd:           req.cmd.clone(),
        volumes:       if host_volumes.is_empty() { None } else { Some(host_volumes) },
        memory_mb:     req.memory_mb,
        cpu_shares:    req.cpu_shares,
        network:       Some(net_name),
    };

    tokio::spawn(async move {
        tracing::info!("Pulling {} for {}", image_bg, name);
        if let Err(e) = docker.pull_image(&image_bg).await {
            tracing::error!("pull_image: {e:?}");
            let _ = queries::update_app_status(&db, id, "error", None).await;
            fire(&db, id, "error", &name).await;
            return;
        }
        match docker.create_container(cfg).await {
            Ok(cid) => match docker.start_container(&cid).await {
                Ok(_) => {
                    let _ = queries::update_app_status(&db, id, "running", Some(&cid)).await;
                    tracing::info!("App {} running on :{}", name, external_port);
                    fire(&db, id, "running", &name).await;
                }
                Err(e) => {
                    tracing::error!("start: {e:?}");
                    let _ = queries::update_app_status(&db, id, "error", Some(&cid)).await;
                    fire(&db, id, "error", &name).await;
                }
            },
            Err(e) => {
                tracing::error!("create: {e:?}");
                let _ = queries::update_app_status(&db, id, "error", None).await;
                fire(&db, id, "error", &name).await;
            }
        }
    });

    (StatusCode::ACCEPTED, Json(json!(AppResponse {
        id, name: req.name.clone(), image,
        status: "pending".to_string(), container_id: None,
        external_port: Some(external_port as i32),
        internal_port: Some(internal_port as i32),
        memory_mb:     req.memory_mb.map(|v| v as i32),
        cpu_shares:    req.cpu_shares.map(|v| v as i32),
    }))).into_response()
}

// List apps
pub async fn list_apps(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match queries::list_apps(&state.db).await {
        Ok(apps) => {
            let mut out = Vec::new();
            for app in &apps { out.push(build_response(&state.db, app).await); }
            (StatusCode::OK, Json(json!(out))).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    }
}

// Get info about an app
pub async fn get_app(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match queries::get_app(&state.db, id).await {
        Ok(Some(app)) => (StatusCode::OK, Json(json!(build_response(&state.db, &app).await))).into_response(),
        Ok(None)      => (StatusCode::NOT_FOUND,  Json(json!({"error":"not found"}))).into_response(),
        Err(e)        => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":e.to_string()}))).into_response(),
    }
}

pub async fn delete_app(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let app = match queries::get_app(&state.db, id).await {
        Ok(Some(a)) => a,
        Ok(None)    => return (StatusCode::NOT_FOUND, Json(json!({"error":"not found"}))).into_response(),
        Err(e)      => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":e.to_string()}))).into_response(),
    };
    if let Ok(ports) = queries::get_port_mappings(&state.db, id).await {
        for p in ports { state.ports.release(p.external_port as u16); }
    }
    if let Some(ref cid) = app.container_id {
        let cid = cid.clone();
        let docker = state.docker.clone();
        tokio::spawn(async move {
            let _ = docker.stop_container(&cid).await;
            let _ = docker.remove_container(&cid).await;
        });
    }
    fire(&state.db, id, "deleted", &app.name).await;
    let _ = queries::delete_port_mappings(&state.db, id).await;
    let _ = queries::delete_app(&state.db, id).await;
    (StatusCode::OK, Json(json!({"message":"deleted"}))).into_response()
}