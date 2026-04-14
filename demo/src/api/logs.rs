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
/// We are not flooding our own servers with our own requests.
const MAX_TAIL_LINES: u64 = 10_000;

#[derive(Debug, Deserialize)]
pub struct LogsQuery {
    pub tail: Option<u64>,
}

// Get container logs
// The container logs you can acces from the shell itself
// But wouldnt it be nice to visualise it?
// That is why this route exists
// You can put in a query and you can view the logs that it will give
// That way I can make it a nice gui in the panel later on
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