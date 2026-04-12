// Imports

// Webhooks let users log events and build automations on top of their apps.
use axum::{
    extract::{Path, State},
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

// Maximum number of webhooks per app — prevents unbounded resource exhaustion.
const MAX_WEBHOOKS_PER_APP: usize = 20;

#[derive(Debug, Deserialize)]
pub struct CreateWebhookRequest {
    pub url: String,
}

/// Validate a webhook URL.
/// - Must be HTTPS (reject HTTP to avoid plaintext exfiltration)
/// - Must not target private/loopback IP ranges (basic SSRF mitigation)
/// - Max 512 characters
fn validate_webhook_url(url: &str) -> Result<(), &'static str> {
    if url.len() > 512 {
        return Err("url must be 512 characters or fewer");
    }
    if !url.starts_with("https://") {
        return Err("webhook url must use HTTPS");
    }
    // Block obvious SSRF targets
    let lower = url.to_lowercase();
    let blocked_hosts = [
        "localhost", "127.", "0.0.0.0", "::1",
        "169.254.",
        "10.", "172.16.", "172.17.", "172.18.", "172.19.",
        "172.20.", "172.21.", "172.22.", "172.23.", "172.24.",
        "172.25.", "172.26.", "172.27.", "172.28.", "172.29.",
        "172.30.", "172.31.", "192.168.",
        "metadata.google", "169.254.169.254",
    ];
    for blocked in &blocked_hosts {
        if lower.contains(blocked) {
            return Err("webhook url targets a private or reserved address");
        }
    }
    Ok(())
}

// List all existing webhooks for an app
pub async fn list_webhooks(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match queries::list_webhooks(&state.db, id).await {
        Ok(wh) => (StatusCode::OK, Json(json!(wh))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    }
}

// Create a new webhook
pub async fn create_webhook(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateWebhookRequest>,
) -> impl IntoResponse {
    if let Err(msg) = validate_webhook_url(&req.url) {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": msg}))).into_response();
    }

    match queries::get_app(&state.db, id).await {
        Ok(None)    => return (StatusCode::NOT_FOUND, Json(json!({"error": "app not found"}))).into_response(),
        Err(e)      => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
        Ok(Some(_)) => {}
    }

    // Enforce per-app webhook limit
    let existing = match queries::list_webhooks(&state.db, id).await {
        Ok(list) => list,
        Err(e)   => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    };
    if existing.len() >= MAX_WEBHOOKS_PER_APP {
        return (StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({"error": format!("maximum {} webhooks per app", MAX_WEBHOOKS_PER_APP)}))).into_response();
    }

    let wh_id = Uuid::new_v4();
    match queries::insert_webhook(&state.db, wh_id, id, &req.url).await {
        Ok(_)  => (StatusCode::CREATED, Json(json!({"id": wh_id, "app_id": id, "url": req.url}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    }
}

// Delete an existing webhook
pub async fn delete_webhook(
    State(state): State<Arc<AppState>>,
    Path((app_id, wh_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    match queries::delete_webhook(&state.db, wh_id, app_id).await {
        Ok(_)  => (StatusCode::OK, Json(json!({"message": "deleted"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    }
}