// Imports

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::json;
use sqlx::query;
use std::sync::Arc;
use uuid::Uuid;

use crate::db::queries;
use crate::server::state::{AppState, PortMappingRecord};

#[derive(Debug, Deserialize)]
pub struct AddPortRequest {
    pub internal_port: u16,
    pub external_port: Option<u16>,
}

// Call this to list the ports assigned
pub async fn list_ports(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match queries::get_port_mappings(&state.db, id).await {
        Ok(ports) => (StatusCode::OK, Json(json!(ports))).into_response(),
        Err(e)    => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    }
}

// Add a port
pub async fn add_port(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<Uuid>,
    Json(req): Json<AddPortRequest>,
) -> impl IntoResponse {
    match queries::get_app(&state.db, app_id).await {
        Ok(None)    => return (StatusCode::NOT_FOUND, Json(json!({"error": "app not found"}))).into_response(),
        Err(e)      => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
        Ok(Some(_)) => {}
    }

    let external = match req.external_port {
        Some(p) => { state.ports.mark_used(p); p }
        None    => state.ports.allocate(),
    };

    let pm = PortMappingRecord {
        id:            Uuid::new_v4(),
        app_id,
        internal_port: req.internal_port as i32,
        external_port: external as i32,
    };

    match queries::insert_port_mapping(&state.db, &pm).await {
        Ok(_)  => (StatusCode::CREATED, Json(json!(pm))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    }
}

// Delete a port through the api
pub async fn delete_port(
    State(state): State<Arc<AppState>>,
    Path((app_id, mapping_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    if let Ok(ports) = queries::get_port_mappings(&state.db, app_id).await {
        if let Some(p) = ports.iter().find(|p: &&PortMappingRecord| p.id == mapping_id) {
            state.ports.release(p.external_port as u16);
        }
    }

    match query("DELETE FROM port_mappings WHERE id=$1 AND app_id=$2")
        .bind(mapping_id)
        .bind(app_id)
        .execute(&state.db)
        .await
    {
        Ok(_)  => (StatusCode::OK, Json(json!({"message": "deleted"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("{e}")}))).into_response(),
    }
}