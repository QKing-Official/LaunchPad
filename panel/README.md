# LaunchPad Panel (Rust)

Minimal Rust control panel for the LaunchPad daemon with:

- First-time setup wizard (creates super admin + core settings)
- Signup/login with cookie sessions
- Role model: `super_admin`, `admin`, `user`
- Multi-node daemon support (no daemon code changes)
- Server create + power actions + logs
- In-browser file path viewer/editor via daemon file API
- Admin settings, users, nodes
- Basic audit logs with 30-day retention

## Stack

- Axum (SSR, server-rendered HTML)
- SQLx + PostgreSQL
- tower-sessions (cookie session)
- reqwest (daemon API)

## Environment

Required variables:

- `DATABASE_URL=postgres://user:pass@127.0.0.1:5432/launchpad_panel`
- `PANEL_BIND=127.0.0.1:4000` (optional)

## Run

```bash
cd panel
cargo run
```

Then open:

- http://127.0.0.1:4000/setup (first run only)

## Daemon Node Setup

In Admin -> Nodes, add each daemon:

- Name
- Base URL (example: `http://127.0.0.1:8000`)
- API key (same as daemon `API_KEY`)

The panel calls daemon APIs using `x-api-key` and uses daemon websocket shell directly via daemon URL + `?key=`.

## Notes

- This is intentionally minimal UI (grayscale) and simple architecture.
- Session storage is in-memory for now. Restarting panel logs users out.
- Add Redis/Postgres-backed sessions later for production.
