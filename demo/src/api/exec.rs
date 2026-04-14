// Imports

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::db::queries;
use crate::server::state::AppState;

// Limits

/// Maximum number of arguments in a single exec request.
const MAX_CMD_ARGS: usize = 64;
/// Maximum length of a single argument string.
const MAX_ARG_LEN: usize = 4096;
/// Maximum stdin payload size (64 KiB).
const MAX_STDIN_BYTES: usize = 64 * 1024;

#[derive(Debug, Deserialize)]
pub struct ExecRequest {
    pub cmd:   Vec<String>,
    pub stdin: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ExecResponse {
    pub output: String,
}

// Execute a command inside a running app container
pub async fn exec_in_app(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<ExecRequest>,
) -> impl IntoResponse {
    // Validation
    if req.cmd.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": "cmd must not be empty"}))).into_response();
    }
    if req.cmd.len() > MAX_CMD_ARGS {
        return (StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("cmd exceeds {} arguments", MAX_CMD_ARGS)}))).into_response();
    }
    for arg in &req.cmd {
        if arg.len() > MAX_ARG_LEN {
            return (StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("argument exceeds {} bytes", MAX_ARG_LEN)}))).into_response();
        }
        if arg.contains('\0') {
            return (StatusCode::BAD_REQUEST,
                Json(json!({"error": "argument must not contain null bytes"}))).into_response();
        }
    }
    if let Some(ref stdin) = req.stdin {
        if stdin.len() > MAX_STDIN_BYTES {
            return (StatusCode::PAYLOAD_TOO_LARGE,
                Json(json!({"error": format!("stdin exceeds {} bytes", MAX_STDIN_BYTES)}))).into_response();
        }
    }

    let app = match queries::get_app(&state.db, id).await {
        Ok(Some(a)) => a,
        Ok(None)    => return (StatusCode::NOT_FOUND, Json(json!({"error": "app not found"}))).into_response(),
        Err(e)      => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    };

    let container_id = match app.container_id {
        Some(ref c) => c.clone(),
        None    => return (StatusCode::CONFLICT, Json(json!({"error": "container not running"}))).into_response(),
    };

    if app.status != "running" {
        return (StatusCode::CONFLICT, Json(json!({"error": format!("app is {}", app.status)}))).into_response();
    }

    match state.docker.exec(&container_id, req.cmd, req.stdin).await {
        Ok(output) => (StatusCode::OK, Json(json!(ExecResponse { output }))).into_response(),
        Err(e)     => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    }
}