// Imports

use axum::{
    response::Html,
    routing::get,
    middleware,
    Router,
};
use std::sync::Arc;

use crate::server::state::AppState;
use super::{
    apps::{get_app, list_apps},
    auth::auth_middleware,
    monitoring::get_stats,
    servers::server_info,
    websocket::ws_shell,
};

// THE ROUTER
// This is the demo. The demo is the locked to prevent abuse. This is pure for testing purposes.
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
                .route("/", get(root))
                .route("/health", get(health))
        .route("/servers", get(server_info))

                .route("/apps", get(list_apps))
                .route("/apps/{id}", get(get_app))
        .route("/apps/{id}/stats", get(get_stats))

        .route_layer(middleware::from_fn_with_state(state.clone(), auth_middleware))

        .route("/apps/{id}/shell", get(ws_shell))

        .with_state(state)
}

async fn root() -> Html<String> {
    let api_key = std::env::var("API_KEY").unwrap_or_default();
    Html(DEMO_PAGE.replace("__API_KEY__", &api_key))
}

async fn health() -> &'static str { "ok" }

const DEMO_PAGE: &str = r#"<!doctype html>
<html lang="en">
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>LaunchPad Daemon Demo</title>
    <style>
        :root {
            --bg: #0f1115;
            --panel: #171a21;
            --panel-2: #1d212a;
            --ink: #e8ecf1;
            --muted: #a6b0bd;
            --line: #2a303c;
            --btn: #2f6feb;
            --btn-2: #2b313d;
        }
        * { box-sizing: border-box; }
        body {
            margin: 0;
            font-family: "Segoe UI", sans-serif;
            color: var(--ink);
            background: var(--bg);
            min-height: 100vh;
        }
        .wrap {
            max-width: 860px;
            margin: 0 auto;
            padding: 16px;
            display: grid;
            gap: 10px;
        }
        .card {
            background: var(--panel);
            border: 1px solid var(--line);
            border-radius: 10px;
            padding: 12px;
        }
        h1 { margin: 0 0 6px; font-size: 1.1rem; }
        .sub { margin: 0; color: var(--muted); }
        .row { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
        label { font-size: .8rem; color: var(--muted); display: block; margin-bottom: 4px; }
        input {
            width: 100%;
            padding: 8px;
            border: 1px solid var(--line);
            border-radius: 8px;
            background: var(--panel-2);
            color: var(--ink);
        }
        .btns { display: flex; flex-wrap: wrap; gap: 6px; margin-top: 8px; }
        button {
            border: 0;
            border-radius: 8px;
            padding: 8px 10px;
            background: var(--btn);
            color: #fff;
            cursor: pointer;
            font-weight: 600;
            font-size: .86rem;
        }
        button.alt { background: var(--btn-2); }
        pre {
            margin: 0;
            min-height: 180px;
            max-height: 55vh;
            overflow: auto;
            border: 1px solid var(--line);
            border-radius: 8px;
            background: #0c0f14;
            padding: 10px;
            font-family: ui-monospace, monospace;
            font-size: .85rem;
        }
        @media (max-width: 700px) {
            .row { grid-template-columns: 1fr; }
        }
    </style>
</head>
<body>
    <main class="wrap">
        <section class="card">
            <h1>LaunchPad Demo</h1>
            <p class="sub">Here you can try out the LaunchPad Daemon.</p>
            <p class="sub">Many features are locked to ensure my privacy and safety</p>
            <p class="sub">You can get a feeling here, but for all information check the repo</p>
            <p class="sub">I highly advise you to selfhost it since there are many cool features like webhooks and power actions.</p>
        </section>

        <section class="card">
            <div class="row">
                <div><label>API Key</label><input id="apiKey" value="__API_KEY__" /></div>
                <div><label>App ID</label><input id="appId" placeholder="auto" /></div>
                <div><label>WS Command</label><input id="wsCmd" value="ls -la" /></div>
            </div>
            <div class="btns">
                <button onclick="callApi('/health')">Health</button>
                <button onclick="callApi('/servers')">Servers</button>
                <button onclick="callApi('/apps')">List Apps</button>
                <button onclick="callApi('/apps/' + appId())">Get App</button>
                <button onclick="callApi('/apps/' + appId() + '/stats')">App Stats</button>
            </div>
            <div class="btns">
                <button class="alt" onclick="pickFirstApp(true)">Refresh App ID</button>
                <button class="alt" onclick="openWs()">Open WS</button>
                <button class="alt" onclick="sendWs()">Send WS Cmd</button>
                <button class="alt" onclick="closeWs()">Close WS</button>
            </div>
        </section>

        <section class="card"><pre id="out">ready</pre></section>
    </main>

    <script>
        let ws = null;
        const out = document.getElementById('out');
        const apiKeyInput = document.getElementById('apiKey');
        const appIdInput = document.getElementById('appId');
        const apiKey = () => document.getElementById('apiKey').value.trim();
        const appId = () => document.getElementById('appId').value.trim();
        const wsCmd = () => document.getElementById('wsCmd').value.trim();
        const log = (v) => {
            out.textContent += '' + (typeof v === 'string' ? v : JSON.stringify(v, null, 2));
            out.scrollTop = out.scrollHeight;
        };

        async function callApi(path) {
            try {
                const headers = {};
                if (path !== '/health' && path !== '/') headers['x-api-key'] = apiKey();
                const res = await fetch(path, { headers });
                const txt = await res.text();
                let payload = txt;
                try { payload = JSON.parse(txt); } catch (_) {}
                out.textContent = '';
                log({ status: res.status, path, payload });
            } catch (e) {
                log('request error: ' + e.message);
            }
        }

        async function pickFirstApp(showLog = false) {
            try {
                const res = await fetch('/apps', { headers: { 'x-api-key': apiKey() } });
                const payload = await res.json();
                if (!Array.isArray(payload) || payload.length === 0) {
                    if (showLog) log('no apps found yet');
                    return;
                }
                appIdInput.value = payload[0].id;
                if (showLog) log('app id set: ' + payload[0].id);
            } catch (e) {
                if (showLog) log('failed to fetch apps: ' + e.message);
            }
        }

        function wsUrl() {
            const proto = location.protocol === 'https:' ? 'wss' : 'ws';
            return `${proto}://${location.host}/apps/${appId()}/shell?key=${encodeURIComponent(apiKey())}`;
        }

        function openWs() {
            if (!appId()) return log('set app id first');
            if (!apiKey()) return log('set api key first');
            if (ws && ws.readyState === WebSocket.OPEN) return log('ws already open');
            ws = new WebSocket(wsUrl());
            ws.onopen = () => {
                out.textContent = '';
                log('ws open');
            };
            ws.onmessage = (ev) => {
                out.textContent = '';
                log(ev.data);
            };
            ws.onerror = () => {
                out.textContent = '';
                log('ws error');
            };
            ws.onclose = () => {
                out.textContent = '';
                log('ws closed');
            };
        }

        function sendWs() {
            if (!ws || ws.readyState !== WebSocket.OPEN) return log('ws not connected');
            const cmd = wsCmd();
            if (!cmd) return log('enter a command');
            out.textContent = '';
            ws.send(cmd);
            log('> ' + cmd);
        }

        function closeWs() {
            if (!ws) return;
            ws.close();
            ws = null;
        }

        (async () => {
            apiKeyInput.value = apiKeyInput.value.trim();
            await pickFirstApp(false);
            out.textContent = 'ready';
            if (appId()) {
                log('auto app id: ' + appId());
            } else {
                log('no app id yet, try Refresh App ID');
            }
        })();
    </script>
</body>
</html>
"#;