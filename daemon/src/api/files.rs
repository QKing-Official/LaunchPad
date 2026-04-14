// Imports
// * Replace this later on, this is the worst shit ever
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

// Sanitisation. Keep your hands clean folks!
// We prevent the basic faults that might crash the daemon
// Isolate everything and string escaping will not be easy
fn validate_path(path: &str) -> Result<(), &'static str> {
    if path.is_empty() {
        return Err("path must not be empty");
    }
    if path.contains('\0') {
        return Err("path must not contain null bytes");
    }
    Ok(())
}

// Validate a filename and only reject null bytes
// null bytes can give an error or can be used to flood the daemon
// Because of that, we prevent it.
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
// For files we use exec in the background secretly
// I have to patch this to prevent floods
// and bigger uploads
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
    // Please turn this into sftp later on
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
            // Also make this sftp
            match state.docker.exec(&cid, vec!["cat".into(), "--".into(), path.clone()], None).await {
                Ok(contents) => (StatusCode::OK, Json(json!({"type":"file","path":path,"contents":contents}))).into_response(),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":e.to_string()}))).into_response(),
            }
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":e.to_string()}))).into_response(),
    }
}


// Uploading files
// This is once again done with exec
// We take the raw data and put it through an exec statement
// This is not safe and I need to patch this
// SFTP should be a way better solution
// So I need to figure out what SFTP does behind the scenes
pub async fn upload_file(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Query(q): Query<FileQuery>,
    body: Bytes,
) -> impl IntoResponse {
    // Enforce a reasonable upload size limit (10 MiB) (soon more due sftp)
    // I can increase this when I turn the the dark side and use sftp
    // So:
    // TODO: ADD FUCKING SFTP!!!!!!!!!!!!!!!!!!!!!
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
    // This is prone to security risks.
    // TODO: Add fucking sftp. I cant keep doing this all day. And sorry for the reviewer for those crazy comments
    // TODO: Write better comments and just prevent letting the reviewer fall to sleep
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
// For the deletion of files we use rm behind the scenes.
// This is not perfect at all
// But holy shit I dont know how to replace this
// So dont expect this to be removed....
// Sorry in advance
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
    // And use rm -rf
    // Forcing file removal wasn't the best idea
    // But ehhhh, I stick to it
    // This isnt really my priority atm
    match state.docker.exec(
        &cid,
        vec!["rm".into(), "-rf".into(), "--".into(), path.clone()],
        None,
    ).await {
        Ok(_)  => (StatusCode::OK, Json(json!({"message":"deleted","path":path}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":e.to_string()}))).into_response(),
    }
}