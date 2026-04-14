// Imports

// This file exists for safety. Only authorised callers may access the API.
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

/// Return the API key from the environment.
fn require_api_key() -> Result<String, StatusCode> {
    env::var("API_KEY").map_err(|_| {
        tracing::error!("API_KEY not set – rejecting request");
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

// Middleware
pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    if req.uri().path() == "/health" || req.uri().path() == "/" {
        return Ok(next.run(req).await);
    }

    let global_key = require_api_key()?;

    let provided = req.headers()
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if constant_time_eq(provided.as_bytes(), global_key.as_bytes()) {
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

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}