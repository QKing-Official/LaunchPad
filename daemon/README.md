# LaunchPad Daemon

This is the daemon for Launchpad.
This daemon is build on top of docker with use of bollard.

What features does it have?
Check the API routes yourself:

## Daemon API Endpoints

### System

| Method | Path     | Description |
|--------|----------|-------------|
| GET    | `/`      | Liveness check — returns "daemon alive" _(No auth required)_ |
| GET    | `/health`| Health check — returns "ok" _(No auth required)_ |
| GET    | `/servers` | Node info — Docker version, CPU, RAM, container counts |

---

### Apps

| Method | Path | Description |
|--------|------|-------------|
| GET | `/apps` | List all apps |
| POST | `/apps` | Create app + pull image + start container (async) <br> _Body: name, image?, internal_port?, external_port?, env?, cmd?, volumes?, memory_mb?, cpu_shares?_ |
| GET | `/apps/{id}` | Get single app |
| DELETE | `/apps/{id}` | Delete app, stop + remove container, release port |

---

### Power

| Method | Path | Description |
|--------|------|-------------|
| POST | `/apps/{id}/power` | start · stop · restart · kill <br> _Body: action, signal? (kill only) — fires webhooks on success_ |

---

### Observability

| Method | Path | Description |
|--------|------|-------------|
| GET | `/apps/{id}/logs` | Container stdout/stderr <br> _Query: tail (default 100)_ |
| GET | `/apps/{id}/stats` | Live CPU %, RAM, network I/O, block I/O |

---

### Exec & Shell

| Method | Path | Description |
|--------|------|-------------|
| POST | `/apps/{id}/exec` | Run a command inside the container, return output <br> _Body: cmd[], stdin? — app must be running_ |
| WS | `/apps/{id}/shell` | Interactive PTY shell over WebSocket <br> _Auth via ?key= query param, not header_ |

---

### Ports

| Method | Path | Description |
|--------|------|-------------|
| GET | `/apps/{id}/ports` | List port mappings |
| POST | `/apps/{id}/ports` | Add a port mapping (DB only — does not rebind Docker) <br> _Body: internal_port, external_port?_ |
| DELETE | `/apps/{id}/ports/{mapping_id}` | Remove a port mapping, release external port |

---

### Files

| Method | Path | Description |
|--------|------|-------------|
| GET | `/apps/{id}/files` | List directory or read file inside container <br> _Query: path (default /)_ |
| POST | `/apps/{id}/files` | Upload a file into the container <br> _Query: path (dir, default /data), name — body: raw bytes_ |
| DELETE | `/apps/{id}/files` | Delete a file or directory (rm -rf) <br> _Query: path — refuses /_ |

---

### Network

| Method | Path | Description |
|--------|------|-------------|
| GET | `/apps/{id}/network` | Get app's Docker network name |
| POST | `/apps/{id}/network/connect` | Connect another app's container to this app's network <br> _Body: target_app_id_ |
| POST | `/apps/{id}/network/disconnect` | Disconnect target app from this app's network <br> _Body: target_app_id_ |

---

### Webhooks

| Method | Path | Description |
|--------|------|-------------|
| GET | `/apps/{id}/webhooks` | List registered webhooks |
| POST | `/apps/{id}/webhooks` | Register a webhook URL for status events <br> _Body: url — fires on running, stopped, error, deleted_ |
| DELETE | `/apps/{id}/webhooks/{wh_id}` | Remove a webhook |

---

### Tokens

| Method | Path | Description |
|--------|------|-------------|
| GET | `/apps/{id}/tokens` | List per-app tokens |
| POST | `/apps/{id}/tokens` | Create a scoped token (only valid for this app's endpoints) <br> _Body: label? — token value shown once, format: lp_…_ |
| DELETE | `/apps/{id}/tokens/{tok_id}` | Revoke a token |

## My journey

I wanted to make a host where people could get a free container to host their HackClub project on (I'm not affiliated with HackClub).
Since hackclub is coding related I just cant use an existing panel in my mindset.
That is why I am writing this daemon and I am going to build a panel on top of it.
This daemon is written in Rust and can compile to single binary.
This uses postgressql and is pretty optimised I must say.
I am also submitting this into the lockin sidequest of HackClub.

This is the solution, a custom daemon with all features I need.
Some are weird I must say. For example webhooks and the network isolation.
I wanted it to be secure so I got to work.

If you are reading this it is probably not done yet, but it works.
I advise you to not use this in production at the time I write this.
This readme will be updated and there will be stated otherwise when it is fully done.

