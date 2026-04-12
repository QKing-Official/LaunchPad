// Imports

// Yeah, webhooks. I am going to attach webhooks and webhook configs for each user action.
// This makes it that the user can easily log and build automations on top of their server
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

#[derive(Debug, Deserialize)]
pub struct CreateWebhookRequest {
    pub url: String,
}

// List all existing webhooks
pub async fn list_webhooks(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match queries::list_webhooks(&state.db, id).await {
        Ok(wh) => (StatusCode::OK, Json(json!(wh))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    }
}

// Creata a new webhook
pub async fn create_webhook(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateWebhookRequest>,
) -> impl IntoResponse {
    match queries::get_app(&state.db, id).await {
        Ok(None)    => return (StatusCode::NOT_FOUND, Json(json!({"error": "app not found"}))).into_response(),
        Err(e)      => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
        Ok(Some(_)) => {}
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