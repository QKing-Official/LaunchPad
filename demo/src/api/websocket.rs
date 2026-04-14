// Imports

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;
use std::env;

use crate::db::queries;
use crate::server::state::AppState;

#[derive(Debug, Deserialize)]
pub struct WsQuery {
    pub key: Option<String>,
}

fn allowed_ws_command(raw: &str) -> Result<Vec<String>, &'static str> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("empty command");
    }
    if trimmed.len() > 120 {
        return Err("command too long");
    }

    let parts: Vec<String> = trimmed
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    if parts.is_empty() {
        return Err("empty command");
    }

    if parts.iter().any(|p| p.chars().any(|c| c.is_control())) {
        return Err("invalid characters in command");
    }

    let cmd = parts[0].as_str();
    let arg_len = parts.len().saturating_sub(1);

    match cmd {
        "ls" => {
            if arg_len <= 2 {
                Ok(parts)
            } else {
                Err("ls accepts up to two arguments")
            }
        }
        "pwd" | "whoami" | "date" | "id" => {
            if arg_len == 0 {
                Ok(parts)
            } else {
                Err("this command does not accept arguments")
            }
        }
        "uname" => {
            if arg_len == 0 || (arg_len == 1 && parts[1] == "-a") {
                Ok(parts)
            } else {
                Err("uname only allows optional -a")
            }
        }
        _ => Err("blocked command (allowed: ls, pwd, whoami, date, uname, id)"),
    }
}

// Broken when used directly in the CLI. DO NOT DARE TO DO THAT!
// You can excecute all shell commands
// This is how you can execute commands through the API
pub async fn ws_shell(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Query(q): Query<WsQuery>,
) -> impl IntoResponse {
    let expected = env::var("API_KEY").unwrap_or_else(|_| "supersecret123".to_string());
    if q.key.as_deref() != Some(&expected) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    ws.on_upgrade(move |socket| handle_socket(socket, state, id))
}

// The socket for the websocket shell
// DO NOT TOUCH
async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>, id: Uuid) {
    let container_id = match queries::get_app(&state.db, id).await {
        Ok(Some(ref app)) if app.status == "running" => {
            match app.container_id {
                Some(ref c) => c.clone(),
                None => {
                    let _ = socket.send(Message::Text("ERROR: no container\n".into())).await;
                    return;
                }
            }
        }
        _ => {
            let _ = socket.send(Message::Text("ERROR: app not found or not running\n".into())).await;
            return;
        }
    };

    let short = container_id[..12].to_string();
    let _ = socket.send(Message::Text(format!("Connected to {}\n", short).into())).await;

    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Text(text) => {
                let argv = match allowed_ws_command(&text) {
                    Ok(v) => v,
                    Err(msg) => {
                        let _ = socket.send(Message::Text(format!("ERROR: {}\n", msg).into())).await;
                        continue;
                    }
                };

                match state.docker.exec(&container_id, argv, None).await {
                    Ok(out) => { let _ = socket.send(Message::Text(out.into())).await; }
                    Err(e)  => { let _ = socket.send(Message::Text(format!("ERROR: {}\n", e).into())).await; }
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
}