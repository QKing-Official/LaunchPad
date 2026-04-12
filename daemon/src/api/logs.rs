// Imports

use axum::{
    extract::{Path, Query, State},
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

/// Maximum lines returnable in a single request.
const MAX_TAIL_LINES: u64 = 10_000;

#[derive(Debug, Deserialize)]
pub struct LogsQuery {
    pub tail: Option<u64>,
}

// Get container logs
pub async fn get_logs(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Query(q): Query<LogsQuery>,
) -> impl IntoResponse {
    let tail = match q.tail {
        Some(n) if n > MAX_TAIL_LINES => {
            return (StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("tail must be ≤ {}", MAX_TAIL_LINES)}))).into_response();
        }
        Some(n) => Some(n),
        None    => Some(100),
    };

    let app = match queries::get_app(&state.db, id).await {
        Ok(Some(a)) => a,
        Ok(None)    => return (StatusCode::NOT_FOUND, Json(json!({"error": "not found"}))).into_response(),
        Err(e)      => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    };

    let cid = match app.container_id {
        Some(ref c) => c.clone(),
        None    => return (StatusCode::CONFLICT, Json(json!({"error": "no container"}))).into_response(),
    };

    match state.docker.logs(&cid, tail).await {
        Ok(output) => (StatusCode::OK, Json(json!({"logs": output}))).into_response(),
        Err(e)     => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    }
}