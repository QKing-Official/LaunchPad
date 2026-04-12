// Imports

// This file exist for safety. I dont want everyone being able to acces the full API
// DONT MODIFY PLEASE
use axum::{
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
    extract::State,
};
use std::{env, sync::Arc};
use uuid::Uuid;

use crate::db::queries;
use crate::server::state::AppState;

// Middleware
pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    if req.uri().path() == "/health" || req.uri().path() == "/" {
        return Ok(next.run(req).await);
    }

    // Global api key
    
    let global_key = env::var("API_KEY").unwrap_or_else(|_| "supersecret123".to_string());

    let provided = req.headers()
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if provided == global_key {
        return Ok(next.run(req).await);
    }

    // Per-app token for safety
    if !provided.is_empty() {
        let path  = req.uri().path().to_string();
        let parts: Vec<&str> = path.splitn(4, '/').collect();
        if parts.len() >= 3 && parts[1] == "apps" {
            if let Ok(app_id) = parts[2].parse::<Uuid>() {
                if let Ok(true) = queries::validate_token(&state.db, provided, app_id).await {
                    return Ok(next.run(req).await);
                }
            }
        }
    }

    Err(StatusCode::UNAUTHORIZED)
}