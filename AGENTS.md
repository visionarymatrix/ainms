# AGENTS.md — Project Memory

## Project Overview
**AINMS** — Workplace accountability system with:
- **Backend**: Go (chi router) on port 8440
- **Admin**: Next.js on port 3440
- **Agent**: Rust, cross-compiled for Windows (`x86_64-pc-windows-gnu`)
- **Public IP**: 173.249.47.143

## Super Admin Credentials
- **Email**: superadmin@ainms.io
- **Password**: changeme
- **JWT Secret**: ainms-dev-secret-change-in-production

## Tmux Sessions
| Session  | Purpose                          | Command                                      |
|----------|----------------------------------|----------------------------------------------|
| `backend`| Go API server                     | `cd /home/dev/projects/sync/backend && go run ./cmd/server` |
| `admin`  | Next.js dev server               | `cd /home/dev/projects/sync/admin && npx next dev -p 3440 -H 0.0.0.0` |

## Key URLs
- Backend API: `http://173.249.47.143:8440`
- Admin UI: `http://173.249.47.143:3440`
- Agent download: `http://173.249.47.143:8440/v1/agent/download?os=windows&arch=amd64`
- Socket.IO: `ws://173.249.47.143:8440/socketio/`

## Database
- PostgreSQL (via pgx)
- ClickHouse (events)
- Redis (sessions)

## Architecture Notes

### Socket.IO Real-Time Communication
- **Go library**: `zishang520/socket.io` v3 (Socket.IO v4 protocol)
- **Rust library**: `rust_socketio` 0.6 (async mode)
- **JS library**: `socket.io-client`
- Socket.IO runs on same port 8440 alongside chi router via `http.ServeMux`
- Path: `/socketio/`
- Agent connects with query params: `?token={install_token}&type=agent&device_id={device_id}`
- Admin connects with query params: `?token={jwt}&type=admin`
- Auth: Agent validates install_token, Admin validates JWT
- Rooms: `device:{deviceID}` for agents, `company:{companyID}:admins` for admins
- Events: `screenshot_request`, `screenshot_ready`, `device_online`, `device_offline`

### Screenshot Flow
1. Admin clicks Screenshot → HTTP POST `/v1/screenshot/request` (creates DB record + pending_command + Socket.IO event to agent)
2. Agent receives `screenshot_request` via Socket.IO (or polls `/v1/devices/{id}/commands` as fallback)
3. Agent captures screenshot using `agent-screenshot` crate
4. Agent uploads via HTTP POST `/v1/screenshot/upload` (multipart: image + request_id + device_id)
5. Backend saves PNG to `public/screenshots/{request_id}.png`, updates DB
6. Backend broadcasts `screenshot_ready` via Socket.IO to company admins
7. Admin displays screenshot inline with fullscreen view

### Heartbeat + Version
- Agent sends `PUT /v1/devices/{id}/heartbeat` with JSON body `{"agent_version": "0.1.0"}`
- Backend stores `agent_version` in devices table
- Admin devices page shows agent version in table and detail view

### HTTP Fallback Endpoints (kept for backward compatibility)
- `GET /v1/devices/{id}/commands` — agent polls for pending commands
- `POST /v1/commands/ack` — acknowledge command
- `PUT /v1/devices/{id}/heartbeat` — device heartbeat
- `GET /v1/devices/{id}/screenshots` — list screenshots for device
- `GET /v1/screenshots/{id}/image` — serve screenshot image
- `POST /v1/screenshot/upload` — agent uploads screenshot image

## Build Commands

### Backend
```bash
cd /home/dev/projects/sync/backend && go run ./cmd/server
```

### Admin
```bash
cd /home/dev/projects/sync/admin && npx next dev -p 3440 -H 0.0.0.0
```

### Agent (Windows cross-compile)
```bash
cd /home/dev/projects/sync/agent && cargo build --release --target x86_64-pc-windows-gnu -p agent-core
```

### Agent binary is symlinked
```
backend/public/agents/ainms-agent_windows_amd64.exe → agent/target/x86_64-pc-windows-gnu/release/agent-core.exe
```

## Known Issues
- Screenshot capture on Windows fails silently — the `ScreenshotCommander::capture()` function in `agent-screenshot` crate does not produce a valid PNG on the Windows machine. The Socket.IO and HTTP communication layers work correctly.