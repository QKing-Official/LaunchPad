// Imports

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
pub struct ConnectRequest {
    pub target_app_id: Uuid,
}

// Get network information for an app
// For example if its created and for what container
pub async fn get_network(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let app = match queries::get_app(&state.db, id).await {
        Ok(Some(a)) => a,
        Ok(None)    => return (StatusCode::NOT_FOUND, Json(json!({"error": "not found"}))).into_response(),
        Err(e)      => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    };
    // Network name is derived from the validated app name (alphanumeric + hyphen + underscore only)
    let network_name = format!("launchpad_{}", app.name);
    (StatusCode::OK, Json(json!({
        "app":     app.name,
        "network": network_name,
    }))).into_response()
}

// Connect
// At the moment you cant communicate between containers
// The networks are isolated from each other
// We need to solve that
// Thats why I added network connections
// With this you can make a network where 2 or more containers are placed on
// This makes you able to send requests between them
pub async fn connect_apps(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<ConnectRequest>,
) -> impl IntoResponse {
    // Prevent connecting an app to itself
    if id == req.target_app_id {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": "source and target app must differ"}))).into_response();
    }

    let app_a = match queries::get_app(&state.db, id).await {
        Ok(Some(a)) => a,
        Ok(None)    => return (StatusCode::NOT_FOUND, Json(json!({"error": "source app not found"}))).into_response(),
        Err(e)      => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    };
    let app_b = match queries::get_app(&state.db, req.target_app_id).await {
        Ok(Some(a)) => a,
        Ok(None)    => return (StatusCode::NOT_FOUND, Json(json!({"error": "target app not found"}))).into_response(),
        Err(e)      => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    };

    let cid_b = match app_b.container_id {
        Some(c) => c,
        None    => return (StatusCode::CONFLICT, Json(json!({"error": "target has no container"}))).into_response(),
    };

    let network_a = format!("launchpad_{}", app_a.name);

    let _ = state.docker.ensure_network(&network_a).await;

    match state.docker.connect_network(&network_a, &cid_b).await {
        Ok(_)  => (StatusCode::OK, Json(json!({"message": format!("{} connected to network {}", app_b.name, network_a)}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    }
}

// Disconnect two apps
// Do you hate the other person to which you connected?
// We got you!
// This allows you to break the shared network.
// You will be put on a isolated network again.
// Problem solved once again

pub async fn disconnect_apps(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<ConnectRequest>,
) -> impl IntoResponse {
    if id == req.target_app_id {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": "source and target app must differ"}))).into_response();
    }

    let app_a = match queries::get_app(&state.db, id).await {
        Ok(Some(a)) => a,
        Ok(None)    => return (StatusCode::NOT_FOUND, Json(json!({"error": "source app not found"}))).into_response(),
        Err(e)      => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    };
    let app_b = match queries::get_app(&state.db, req.target_app_id).await {
        Ok(Some(a)) => a,
        Ok(None)    => return (StatusCode::NOT_FOUND, Json(json!({"error": "target app not found"}))).into_response(),
        Err(e)      => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    };

    let cid_b    = match app_b.container_id { Some(c) => c, None => return (StatusCode::CONFLICT, Json(json!({"error": "no container"}))).into_response() };
    let network_a = format!("launchpad_{}", app_a.name);

    match state.docker.disconnect_network(&network_a, &cid_b).await {
        Ok(_)  => (StatusCode::OK, Json(json!({"message": "disconnected"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    }
}