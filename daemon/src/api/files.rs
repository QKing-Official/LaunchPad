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

// Sanitisation
fn validate_path(path: &str) -> Result<(), &'static str> {
    if path.is_empty() {
        return Err("path must not be empty");
    }
    if path.contains('\0') {
        return Err("path must not contain null bytes");
    }
    Ok(())
}

/// Validate a filename and only reject null bytes.
fn validate_filename(name: &str) -> Result<(), &'static str> {
    if name.is_empty() {
        return Err("filename must not be empty");
    }
    if name.contains('\0') {
        return Err("filename must not contain null bytes");
    }
    Ok(())
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

    if let Err(msg) = validate_path(&path) {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": msg}))).into_response();
    }

    // Use an array form of exec to avoid shell injection
    let check = state.docker.exec(
        &cid,
        vec!["sh".into(), "-c".into(),
             format!("test -d '{}' && echo dir || echo file",
                     path.replace('\'', "'\\''"))],
        None,
    ).await;

    match check {
        Ok(kind) if kind.trim() == "dir" => {
            // Pass path as an argument, not interpolated into a shell string
            match state.docker.exec(
                &cid,
                vec!["ls".into(), "-la".into(), "--".into(), path.clone()],
                None,
            ).await {
                Ok(listing) => (StatusCode::OK, Json(json!({"type":"directory","path":path,"listing":listing}))).into_response(),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":e.to_string()}))).into_response(),
            }
        }
        Ok(_) => {
            // Pass path as an argument to `cat`, not via shell interpolation
            match state.docker.exec(&cid, vec!["cat".into(), "--".into(), path.clone()], None).await {
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
    // Enforce a reasonable upload size limit (10 MiB)
    const MAX_UPLOAD_BYTES: usize = 10 * 1024 * 1024;
    if body.len() > MAX_UPLOAD_BYTES {
        return (StatusCode::PAYLOAD_TOO_LARGE,
            Json(json!({"error": "upload exceeds 10 MiB limit"}))).into_response();
    }

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

    if let Err(msg) = validate_path(&dir) {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": msg}))).into_response();
    }
    if let Err(msg) = validate_filename(&name) {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": msg}))).into_response();
    }

    let dest = format!("{}/{}", dir.trim_end_matches('/'), name);

    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&body);

    let mkdir_result = state.docker.exec(
        &cid,
        vec!["mkdir".into(), "-p".into(), "--".into(), dir.clone()],
        None,
    ).await;

    if let Err(e) = mkdir_result {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":e.to_string()}))).into_response();
    }

    // Write decoded bytes directly; pass b64 as stdin, not as a shell argument.
    let cmd = format!("base64 -d > '{}'", dest.replace('\'', "'\\''"));
    match state.docker.exec(
        &cid,
        vec!["sh".into(), "-c".into(), cmd],
        Some(b64),
    ).await {
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

    if let Err(msg) = validate_path(&path) {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": msg}))).into_response();
    }

    // Pass path as an exec argument
    match state.docker.exec(
        &cid,
        vec!["rm".into(), "-rf".into(), "--".into(), path.clone()],
        None,
    ).await {
        Ok(_)  => (StatusCode::OK, Json(json!({"message":"deleted","path":path}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":e.to_string()}))).into_response(),
    }
}