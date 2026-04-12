// Imports

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
use crate::server::state::AppState;

#[derive(Debug, Deserialize)]
pub struct PowerRequest {
    pub action: String,
    pub signal: Option<String>,
}

/// Allowed POSIX signal names for the `kill` action.
const ALLOWED_SIGNALS: &[&str] = &[
    "SIGKILL", "SIGTERM", "SIGHUP", "SIGUSR1", "SIGUSR2", "SIGINT",
];

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
                .danger_accept_invalid_certs(false)
                .build().unwrap_or_default();
            match client.post(&url).json(&body).send().await {
                Ok(r)  => tracing::info!("Webhook {} -> {}", url, r.status()),
                Err(e) => tracing::warn!("Webhook {} failed: {}", url, e),
            }
        });
    }
}

// Perform power actions: start / stop / restart / kill
pub async fn power_action(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<PowerRequest>,
) -> impl IntoResponse {
    // Validate action early
    if !["start", "stop", "restart", "kill"].contains(&req.action.as_str()) {
        return (StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("unknown action '{}', use: start|stop|restart|kill", req.action)}))).into_response();
    }

    // Validate signal if provided
    if req.action == "kill" {
        let signal = req.signal.as_deref().unwrap_or("SIGKILL");
        if !ALLOWED_SIGNALS.contains(&signal) {
            return (StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("signal '{}' is not allowed; permitted: {:?}", signal, ALLOWED_SIGNALS)}))).into_response();
        }
    }

    let app = match queries::get_app(&state.db, id).await {
        Ok(Some(a)) => a,
        Ok(None)    => return (StatusCode::NOT_FOUND, Json(json!({"error": "not found"}))).into_response(),
        Err(e)      => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    };

    let cid = match app.container_id {
        Some(ref c) => c.clone(),
        None => return (StatusCode::CONFLICT, Json(json!({"error": "no container assigned yet"}))).into_response(),
    };

    let (result, new_status) = match req.action.as_str() {
        "start" => {
            let r = state.docker.start_container(&cid).await;
            if r.is_ok() {
                let _ = queries::update_app_status(&state.db, id, "running", Some(&cid)).await;
            }
            (r.map_err(|e| e.to_string()), "running")
        }
        "stop" => {
            let r = state.docker.stop_container(&cid).await;
            if r.is_ok() {
                let _ = queries::update_app_status(&state.db, id, "stopped", Some(&cid)).await;
            }
            (r.map_err(|e| e.to_string()), "stopped")
        }
        "restart" => {
            let r = state.docker.restart_container(&cid).await;
            if r.is_ok() {
                let _ = queries::update_app_status(&state.db, id, "running", Some(&cid)).await;
            }
            (r.map_err(|e| e.to_string()), "running")
        }
        "kill" => {
            let signal = req.signal.as_deref().unwrap_or("SIGKILL");
            let r = state.docker.kill_container(&cid, signal).await;
            if r.is_ok() {
                let _ = queries::update_app_status(&state.db, id, "stopped", Some(&cid)).await;
            }
            (r.map_err(|e| e.to_string()), "stopped")
        }
        // Unreachable due to early validation above
        _ => unreachable!(),
    };

    match result {
        Ok(_) => {
            fire(&state.db, id, new_status, &app.name).await;
            (StatusCode::OK, Json(json!({"message": req.action}))).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e}))).into_response(),
    }
}