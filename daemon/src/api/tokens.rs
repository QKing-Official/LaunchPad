// Imports

// This is so we can isolate user based actions and global daemon actions.
// The global api key still works on the user routes for ease of use.
use axum::{
    extract::{Path, State},
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::db::queries;
use crate::server::state::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateTokenRequest {
    pub label: Option<String>,
}

// List tokens
pub async fn list_tokens(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match queries::list_tokens(&state.db, id).await {
        Ok(tokens) => (StatusCode::OK, Json(json!(tokens))).into_response(),
        Err(e)     => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    }
}

// Creat a token
pub async fn create_token(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<Uuid>,
    Json(req): Json<CreateTokenRequest>,
) -> impl IntoResponse {
    match queries::get_app(&state.db, app_id).await {
        Ok(None)    => return (StatusCode::NOT_FOUND, Json(json!({"error": "app not found"}))).into_response(),
        Err(e)      => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
        Ok(Some(_)) => {}
    }

    let token_id    = Uuid::new_v4();
    let token_value = format!("lp_{}", Uuid::new_v4().to_string().replace('-', ""));
    let label       = req.label.unwrap_or_else(|| "default".to_string());

    match queries::insert_token(&state.db, token_id, app_id, &token_value, &label).await {
        Ok(_)  => (StatusCode::CREATED, Json(json!({
            "id":    token_id,
            "token": token_value,   // only shown once
            "label": label,
            "app_id": app_id,
        }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    }
}

// Delete the token
pub async fn delete_token(
    State(state): State<Arc<AppState>>,
    Path((app_id, tok_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    match queries::delete_token(&state.db, tok_id, app_id).await {
        Ok(_)  => (StatusCode::OK, Json(json!({"message": "revoked"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    }
}

/// Middleware that accepts EITHER the global API key OR a valid per-app token
pub async fn auth_middleware_with_tokens(
    State(state): State<Arc<AppState>>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Always skip auth on health/root since its public
    if req.uri().path() == "/health" || req.uri().path() == "/" {
        return Ok(next.run(req).await);
    }

    let global_key = std::env::var("API_KEY").unwrap_or_else(|_| "supersecret123".to_string());

    let provided = req.headers()
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    // Global key
    // allow everything since its the main key
    if provided == global_key {
        return Ok(next.run(req).await);
    }

    // Per-app token (restricted to apps/<uuid>)

    let path = req.uri().path().to_string();
    let parts: Vec<&str> = path.splitn(4, '/').collect();
    if parts.len() >= 3 && parts[1] == "apps" {
        if let Ok(app_id) = parts[2].parse::<Uuid>() {
            if let Ok(valid) = queries::validate_token(&state.db, provided, app_id).await {
                if valid {
                    return Ok(next.run(req).await);
                }
            }
        }
    }

    Err(StatusCode::UNAUTHORIZED)
}