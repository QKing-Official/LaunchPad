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

/// Valid port range for user-facing external ports.
/// Ports below 1024 are privileged; ports above 65535 are invalid.
/// We further restrict to the unprivileged dynamic range.
/// We dont care about what port the user wants....
const MIN_PORT: u16 = 1024;
const MAX_PORT: u16 = 65535;

#[derive(Debug, Deserialize)]
pub struct AddPortRequest {
    pub internal_port: u16,
    pub external_port: Option<u16>,
}

fn validate_port(port: u16) -> Result<(), &'static str> {
    if port < MIN_PORT {
        return Err("port must be ≥ 1024 (privileged ports are not allowed)");
    }
    // u16 max is 65535 is valid, but it cant be any higher.
    // Maybe I went overkill with tis
    if port > MAX_PORT {
        return Err("port must be ≤ 65535");
    }
    Ok(())
}

// List ports assigned to an app so that the user can retrieve them and properly integrate them
// We intergrate the mapping as well, but in another function
pub async fn list_ports(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match queries::get_port_mappings(&state.db, id).await {
        Ok(ports) => (StatusCode::OK, Json(json!(ports))).into_response(),
        Err(e)    => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    }
}

// Add a port mapping so the user can have for example a service running on port 80
// For example their public port is 80000, so they cant acces it.
// With the port mapping they can mape 80 internal to 80000 external
// That means that the user can make the service publically available on the external point that it's assigned.

pub async fn add_port(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<Uuid>,
    Json(req): Json<AddPortRequest>,
) -> impl IntoResponse {
    if let Err(msg) = validate_port(req.internal_port) {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": msg}))).into_response();
    }
    if let Some(ext) = req.external_port {
        if let Err(msg) = validate_port(ext) {
            return (StatusCode::BAD_REQUEST, Json(json!({"error": msg}))).into_response();
        }
    }

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

// Delete a port mapping
// For example the user has a service on port 443, but the previous mapping was made 80 -> 80000
// That means they cant acces it publically.
// With this function you can delete the existing mapping
// That makes the user able to create a new one
// Problem solved
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