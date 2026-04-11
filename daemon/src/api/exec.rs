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

#[derive(Debug, Deserialize)]
pub struct ExecRequest {
    pub cmd:   Vec<String>,
    pub stdin: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ExecResponse {
    pub output: String,
}

// Execution inside the app (commands, we arent killing people here)
pub async fn exec_in_app(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<ExecRequest>,
) -> impl IntoResponse {
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