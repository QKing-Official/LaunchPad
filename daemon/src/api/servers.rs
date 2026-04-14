// Imports

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use std::sync::Arc;

use crate::server::state::AppState;

// Fetch server information from docker/client.rs
// This info can later be parsed into various kinds of requests
// This is also used in the output when fetching the specific server
pub async fn server_info(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.docker.docker.info().await {
        Ok(info) => {
            let resp = json!({
                "daemon": "online",
                "docker": {
                    "containers_running": info.containers_running,
                    "containers_stopped": info.containers_stopped,
                    "images":             info.images,
                    "server_version":     info.server_version,
                    "os":                 info.operating_system,
                    "architecture":       info.architecture,
                    "ncpu":               info.ncpu,
                    "mem_total":          info.mem_total,
                }
            });
            (StatusCode::OK, Json(resp)).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    }
}