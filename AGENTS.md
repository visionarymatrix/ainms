# AINMS — Agent Reference

Monorepo: workplace accountability system with three components.

## Start Order

1. **Docker Compose first** — backend depends on Postgres/Redis/MinIO. Before starting the backend, verify Docker is running and containers are up:
   ```
   cd backend && make docker-up
   ```
2. **Backend** — runs on `:8440`. On Windows: use `psmux` (not tmux). If psmux is not installed, ask user before running `winget install psmux`.
3. **Admin portal** — `cd admin && npm run dev`, runs on `:3440` (Next.js dev server default is `:3000`, check if overridden).

## Project Structure

- **`admin/`** — Next.js 16 (App Router, TypeScript, Tailwind v4, shadcn/ui "new-york" style). Auth via cookie `ainms_session` + localStorage. Uses socket.io-client for real-time. Path alias `@/*`.
- **`backend/`** — Go 1.26, chi router, pgx (Postgres), clickhouse-go, go-redis, socket.io server. Entry: `cmd/server/main.go`. Config: `configs/config.dev.yaml`. Migrations in `migrations/postgres/` and `migrations/clickhouse/`.
- **`agent/`** — Rust workspace (stable channel), 11 crates. Entry crate: `agent-core`. Cross-platform (Windows/macOS/Linux). Config: TOML. Windows installer: `scripts/install-ainms-agent.ps1` (must run as admin).

## Dev Ports

| Service      | Port |
|-------------|------|
| PostgreSQL  | 5440 |
| Redis       | 6390 |
| MinIO       | 9101 (API), 9102 (Console) |
| Gateway API | 8440 |
| Admin       | 3440 (or 3000) |

## Key Commands

### Backend
```bash
cd backend
make dev           # build + run
make test          # go test -v -race ./...
make lint          # golangci-lint run ./...
make fmt           # gofmt + goimports
make migrate-up    # run Postgres migrations
make docker-up     # start docker-compose from project root
```

### Admin
```bash
cd admin
npm run dev        # Next.js dev server
npm run build      # production build
npm run lint       # eslint
```

### Agent
```bash
cd agent
cargo build        # debug build
cargo build --release  # release build
cargo test         # run tests
```

## Architecture Notes

- **Edge-first design**: Agent classifies locally (rules + ONNX ML), server stores results. No server-side classification.
- **Dual upload channels**: Priority (immediate: alerts, tamper, popups) and Bulk (every ~5h: usage events + summaries).
- **Auth split**: Agent uses install tokens/JWT. Admin portal uses JWT stored in localStorage + `ainms_session` cookie. Middleware in `admin/middleware.ts` protects all routes except `/login` and `/api`.
- **Socket.IO** for real-time agent↔server commands (screenshots, policy updates). Configured in `main.go` with CORS for `localhost:3440`.
- **Two databases**: Postgres (config/transactional), ClickHouse (analytics/time-series). ClickHouse failure is non-fatal — gateway logs a warning and disables analytics endpoints.

## Conventions

- Backend: Go standard project layout (`internal/config`, `handler`, `service`, `repository/postgres`, `repository/clickhouse`, `store/redis`). Chi router with route groups.
- Admin: Server Components by default, Client Components only for interactivity. `lib/api/client.ts` is the HTTP client. `lib/auth/session.ts` manages auth state. shadcn/ui components in `components/ui/`.
- Agent: Rust workspace with `crates/` for main crates and `shared/` for proto/comms libraries. Platform-specific code behind `cfg(target_os)`.
- Migrations are numbered SQL files, not managed by a migration tool version.

## Gotchas

- Backend `Makefile` references `../docker-compose.yml` (one level up) for docker commands.
- ClickHouse is not in docker-compose yet — only Postgres, Redis, MinIO are. Analytics endpoints will fail without it.
- Agent Windows installer uses `sc.exe` via temp `.bat` files (PowerShell 5.1 `sc` alias conflict workaround).
- Admin `NEXT_PUBLIC_API_URL` defaults to `http://localhost:8440` if unset.
- Backend seeds a super admin on startup (`authSvc.SeedSuperAdmin`). Default credentials are in config.