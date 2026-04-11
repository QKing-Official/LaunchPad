// Imports

use axum::{
    body::Bytes,
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

#[derive(Debug, Deserialize)]
pub struct FileQuery {
    pub path: Option<String>,
    pub name: Option<String>,
}

// Listing and reading files

pub async fn list_or_read(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Query(q): Query<FileQuery>,
) -> impl IntoResponse {
    let app = match queries::get_app(&state.db, id).await {
        Ok(Some(a)) => a,
        Ok(None)    => return (StatusCode::NOT_FOUND, Json(json!({"error":"not found"}))).into_response(),
        Err(e)      => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":e.to_string()}))).into_response(),
    };
    let cid = match app.container_id {
        Some(c) => c,
        None    => return (StatusCode::CONFLICT, Json(json!({"error":"no container"}))).into_response(),
    };
    let path = q.path.unwrap_or_else(|| "/".to_string());
    let check = state.docker.exec(&cid,
        vec!["bash".into(), "-c".into(), format!("test -d '{}' && echo dir || echo file", path)],
        None).await;
    match check {
        Ok(kind) if kind.trim() == "dir" => {
            match state.docker.exec(&cid,
                vec!["bash".into(), "-c".into(), format!("ls -la '{}' 2>&1", path)], None).await {
                Ok(listing) => (StatusCode::OK, Json(json!({"type":"directory","path":path,"listing":listing}))).into_response(),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":e.to_string()}))).into_response(),
            }
        }
        Ok(_) => {
            match state.docker.exec(&cid, vec!["cat".into(), path.clone()], None).await {
                Ok(contents) => (StatusCode::OK, Json(json!({"type":"file","path":path,"contents":contents}))).into_response(),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":e.to_string()}))).into_response(),
            }
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":e.to_string()}))).into_response(),
    }
}


// Uploading files
pub async fn upload_file(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Query(q): Query<FileQuery>,
    body: Bytes,
) -> impl IntoResponse {
    let app = match queries::get_app(&state.db, id).await {
        Ok(Some(a)) => a,
        Ok(None)    => return (StatusCode::NOT_FOUND, Json(json!({"error":"not found"}))).into_response(),
        Err(e)      => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":e.to_string()}))).into_response(),
    };
    let cid = match app.container_id {
        Some(c) => c,
        None    => return (StatusCode::CONFLICT, Json(json!({"error":"no container"}))).into_response(),
    };
    let dir  = q.path.unwrap_or_else(|| "/data".to_string());
    let name = q.name.unwrap_or_else(|| "upload".to_string());
    let dest = format!("{}/{}", dir.trim_end_matches('/'), name);

    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&body);
    let cmd = format!("mkdir -p '{}' && echo '{}' | base64 -d > '{}'", dir, b64, dest);

    match state.docker.exec(&cid, vec!["bash".into(), "-c".into(), cmd], None).await {
        Ok(_)  => (StatusCode::CREATED, Json(json!({"message":"uploaded","path":dest}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":e.to_string()}))).into_response(),
    }
}

// Deletion of files
pub async fn delete_file(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Query(q): Query<FileQuery>,
) -> impl IntoResponse {
    let app = match queries::get_app(&state.db, id).await {
        Ok(Some(a)) => a,
        Ok(None)    => return (StatusCode::NOT_FOUND, Json(json!({"error":"not found"}))).into_response(),
        Err(e)      => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":e.to_string()}))).into_response(),
    };
    let cid = match app.container_id {
        Some(c) => c,
        None    => return (StatusCode::CONFLICT, Json(json!({"error":"no container"}))).into_response(),
    };
    let path = match q.path {
        Some(p) => p,
        None    => return (StatusCode::BAD_REQUEST, Json(json!({"error":"path required"}))).into_response(),
    };
    if path == "/" || path.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(json!({"error":"refusing to delete root"}))).into_response();
    }
    match state.docker.exec(&cid, vec!["rm".into(), "-rf".into(), path.clone()], None).await {
        Ok(_)  => (StatusCode::OK, Json(json!({"message":"deleted","path":path}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":e.to_string()}))).into_response(),
    }
}