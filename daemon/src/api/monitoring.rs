// Imports

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::db::queries;
use crate::server::state::AppState;

// Fetch the stats from docker/client.rs and push them to the status function
pub async fn get_stats(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let app = match queries::get_app(&state.db, id).await {
        Ok(Some(a)) => a,
        Ok(None)    => return (StatusCode::NOT_FOUND, Json(json!({"error":"not found"}))).into_response(),
        Err(e)      => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":e.to_string()}))).into_response(),
    };
    let cid = match app.container_id {
        Some(ref c) => c.clone(),
        None => return (StatusCode::CONFLICT, Json(json!({"error":"no container"}))).into_response(),
    };
    if app.status != "running" {
        return (StatusCode::CONFLICT, Json(json!({"error": format!("app is {}", app.status)}))).into_response();
    }

    let s = match state.docker.stats_json(&cid).await {
        Ok(v)  => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":e.to_string()}))).into_response(),
    };

    // CPU %
    let cpu_now  = s["cpu_stats"]["cpu_usage"]["total_usage"].as_u64().unwrap_or(0);
    let cpu_pre  = s["precpu_stats"]["cpu_usage"]["total_usage"].as_u64().unwrap_or(0);
    let sys_now  = s["cpu_stats"]["system_cpu_usage"].as_u64().unwrap_or(0);
    let sys_pre  = s["precpu_stats"]["system_cpu_usage"].as_u64().unwrap_or(0);
    let num_cpus = s["cpu_stats"]["online_cpus"].as_u64().unwrap_or(1) as f64;
    let cpu_pct  = if sys_now > sys_pre {
        (cpu_now.saturating_sub(cpu_pre) as f64 / (sys_now - sys_pre) as f64) * num_cpus * 100.0
    } else { 0.0 };

    // Memory
    let mem_usage = s["memory_stats"]["usage"].as_u64().unwrap_or(0);
    let mem_limit = s["memory_stats"]["limit"].as_u64().unwrap_or(0);
    let mem_cache = s["memory_stats"]["stats"]["cache"].as_u64().unwrap_or(0);
    let mem_rss   = mem_usage.saturating_sub(mem_cache);
    let mem_pct   = if mem_limit > 0 {
        format!("{:.2}", mem_usage as f64 / mem_limit as f64 * 100.0)
    } else { "0.00".into() };

    // Network
    let (net_rx, net_tx) = if let Some(nets) = s["networks"].as_object() {
        nets.values().fold((0u64, 0u64), |(rx, tx), n| (
            rx + n["rx_bytes"].as_u64().unwrap_or(0),
            tx + n["tx_bytes"].as_u64().unwrap_or(0),
        ))
    } else { (0, 0) };

    // Block I/O
    let (blk_r, blk_w) = if let Some(ops) = s["blkio_stats"]["io_service_bytes_recursive"].as_array() {
        ops.iter().fold((0u64, 0u64), |(r, w), op| {
            let v = op["value"].as_u64().unwrap_or(0);
            match op["op"].as_str().unwrap_or("").to_lowercase().as_str() {
                "read"  => (r + v, w),
                "write" => (r, w + v),
                _       => (r, w),
            }
        })
    } else { (0, 0) };

    (StatusCode::OK, Json(json!({
        "app_id":   id,
        "app_name": app.name,
        "status":   "running",
        "cpu": {
            "percent":  format!("{:.2}", cpu_pct),
            "num_cpus": num_cpus as u64,
        },
        "memory": {
            "rss_bytes":   mem_rss,
            "rss_mb":      mem_rss / 1024 / 1024,
            "usage_bytes": mem_usage,
            "usage_mb":    mem_usage / 1024 / 1024,
            "limit_bytes": mem_limit,
            "limit_mb":    mem_limit / 1024 / 1024,
            "percent":     mem_pct,
        },
        "network": {
            "rx_bytes": net_rx,
            "tx_bytes": net_tx,
            "rx_mb":    net_rx / 1024 / 1024,
            "tx_mb":    net_tx / 1024 / 1024,
        },
        "block_io": {
            "read_bytes":  blk_r,
            "write_bytes": blk_w,
        },
    }))).into_response()
}