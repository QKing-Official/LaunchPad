// Imports

use axum::{Router, routing::{delete, get, post}, middleware};
use std::sync::Arc;

use crate::server::state::AppState;
use super::{
    apps::{create_app, delete_app, get_app, list_apps},
    auth::auth_middleware,
    exec::exec_in_app,
    files::{delete_file, list_or_read, upload_file},
    logs::get_logs,
    monitoring::get_stats,
    network::{connect_apps, disconnect_apps, get_network},
    ports::{add_port, delete_port, list_ports},
    power::power_action,
    servers::server_info,
    tokens::{create_token, delete_token as delete_tok, list_tokens},
    webhooks::{create_webhook, delete_webhook, list_webhooks},
    websocket::ws_shell,
};

// THE ROUTER
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/",        get(root))
        .route("/health",  get(health))
        .route("/servers", get(server_info))

        // Apps CRUD
        .route("/apps",      get(list_apps).post(create_app))
        .route("/apps/{id}", get(get_app).delete(delete_app))

        // Power
        .route("/apps/{id}/power", post(power_action))

        // Exec, Logs, Stats
        .route("/apps/{id}/exec",  post(exec_in_app))
        .route("/apps/{id}/logs",  get(get_logs))
        .route("/apps/{id}/stats", get(get_stats))

        // Ports
        .route("/apps/{id}/ports",              get(list_ports).post(add_port))
        .route("/apps/{id}/ports/{mapping_id}", delete(delete_port))

        // Files
        .route("/apps/{id}/files", get(list_or_read).post(upload_file).delete(delete_file))

        // Network
        .route("/apps/{id}/network",            get(get_network))
        .route("/apps/{id}/network/connect",    post(connect_apps))
        .route("/apps/{id}/network/disconnect", post(disconnect_apps))

        // Webhooks
        .route("/apps/{id}/webhooks",         get(list_webhooks).post(create_webhook))
        .route("/apps/{id}/webhooks/{wh_id}", delete(delete_webhook))

        // Tokens
        .route("/apps/{id}/tokens",          get(list_tokens).post(create_token))
        .route("/apps/{id}/tokens/{tok_id}", delete(delete_tok))

        .route_layer(middleware::from_fn_with_state(state.clone(), auth_middleware))

        // WebSocket with own ?key= auth
        .route("/apps/{id}/shell", get(ws_shell))

        .with_state(state)
}

async fn root()   -> &'static str { "daemon alive" }
async fn health() -> &'static str { "ok" }