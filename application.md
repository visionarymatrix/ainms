# AINMS — Product Vision & Architecture

---

## Part 1: Product Vision (Non-Technical)

### What Is AINMS?

AINMS helps business owners and HR teams understand how their employees use company devices — and gently correct unproductive behavior in real time.

Think of it as a **smart workplace accountability system**. It runs quietly on each employee's computer, tracks which applications they use and for how long, and gently nudges them when they drift away from work-related tasks.

---

### How It Works (In Plain Language)

1. **Each employee's computer gets a small, lightweight application installed.** It watches which apps are open, how long they're used, and optionally takes screenshots to verify what's on screen.

2. **The application thinks for itself.** Instead of sending every screenshot and every action to a server far away, it processes things right there on the employee's computer. It knows the rules for that employee's role and checks locally: "Should this person be on YouTube right now?"

3. **Immediate, helpful feedback.** If an employee opens something they shouldn't, a small popup appears on their screen within seconds — not minutes, not hours. The popup isn't just a warning; it asks: "Please explain why you're using this." The employee writes a reason, and that explanation is saved.

4. **Summarized data goes to the boss — with the receipts.** Every few hours, the application sends both a summary and the full metadata to the main server. The summary gives the big picture: "Employee #42 spent 6 hours on productive work, 45 minutes on social media, and was flagged twice." The metadata provides the receipts: exactly which apps, when, for how long, with classification and confidence scores. Dashboards show the summary; drill-down shows every individual event.

5. **The HR or business owner sees a clear dashboard.** They see which employees are productive, which apps are used most, and any flagged incidents with the employee's own explanations attached.

---

### Key Features

#### Real-Time Behavioral Nudges
When someone opens an app that doesn't match their role, they get a popup immediately. Not five minutes later — instantly. This is the single most impactful feature: timely feedback changes behavior.

#### Role-Based Rules
An HR manager defines what each role can and shouldn't use. A marketing person is allowed (even expected) to be on social media. A developer is not. The system respects the difference.

#### Employee Explanations
Popups aren't one-way scolding. The employee can explain why they're using a flagged application. Maybe they had a legitimate reason — a designer watching a tutorial on YouTube, for example. That explanation is recorded and visible to HR.

#### Privacy by Design
The most sensitive analysis happens on the employee's computer. Screenshots are classified locally by a small AI model. Full screenshots are only uploaded if the company policy explicitly requires it. Most of the time, only text summaries leave the device: "Screenshot classified as: entertainment, confidence: 92%."

#### Intelligent App Classification
The system doesn't just match app names against a list. A small AI model on each computer can classify apps and window titles it has never seen before. If a developer opens a new coding tool the company hasn't whitelisted yet, the model recognizes it as productive.

#### Periodic Summaries with Full Metadata
Data is sent to the server in batches every few hours — not as a constant live stream. This reduces network usage, server costs, and the "Big Brother" feeling. However, important events (alerts, explanations, tampering) are sent immediately. Each batch contains both an aggregated summary (for dashboard rollups) and the full per-event metadata (app name, window title, process info, duration, classification, confidence) for drill-down analysis and compliance auditing.

#### Unkillable Agent (Self-Healing & Persistent)
The endpoint agent cannot be stopped by the employee. It registers as a system service with the highest resilience possible: on Windows it installs as a privileged service that rejects stop commands from non-admin users; on macOS it runs as a LaunchDaemon; on Linux it runs as a systemd service with `Restart=always`. A companion watchdog process monitors the agent — if the agent process is killed, the watchdog restarts it within seconds. If the watchdog itself is killed, the agent restarts the watchdog. Killing either one triggers a tamper alert sent on the priority channel. The agent also protects its own binary and configuration files from modification or deletion. The only way to uninstall is through the admin portal pushing a signed uninstall command.

#### On-Demand Remote Screenshots
HR, managers, and admins can take a screenshot of any enrolled device at any time — instantly — from the admin portal. No waiting for the next scheduled capture. The portal sends a command through the priority channel; the agent captures the screen within seconds, classifies it locally, and uploads the image (or metadata-only, depending on policy) back to the server. This is useful for investigations, audits, or spot-checks. Every on-demand capture is logged with who requested it, when, and why (the requester must provide a reason).

#### Company Registration & Employee-First Enrollment
Before any device can connect, the company must be registered in the system and employees must be created in the admin portal first. When the agent installer runs on an employee's machine, it asks for an Employee ID (not an enrollment code). The agent calls the server with this Employee ID; the server looks up the employee, verifies they exist and are active, and only then issues a device certificate and connects the device to that employee's profile. This ensures every device is tied to a known, registered person — no anonymous devices in the system.

#### Device Connection Monitoring
The main server continuously monitors which devices are connected, which have recently disconnected, and which registered employees have not yet enrolled a device. The admin portal shows a live "Device Fleet" view: total devices, online now, last-seen timestamps, devices that haven't reported in over N hours (flagged as potentially offline or tampered), and employees with no enrolled device yet. This gives administrators full visibility over the entire device fleet in real time.

#### Clear HR Dashboard
Owners and HR see at a glance: who's productive, who's struggling, which departments need attention. The dashboard shows aggregated metrics, trend charts, flagged incidents, and the employee explanations for each flag.

#### Employee Transparency
Employees know the system is there. They can see their own productivity data on a personal "My Data" page. This isn't spyware — it's an accountability tool that works best when everyone understands the rules.

---

### What Makes This Different

| Traditional Monitoring             | AINMS                                       |
| ---------------------------------- | ------------------------------------------- |
| Records everything, sends it all to a server | Processes data locally, sends both metadata AND summaries to server |
| Alerts arrive minutes or hours late | Popups appear in under a second             |
| "Caught you" surveillance       | "Hey, please explain" — accountability with empathy |
| Every screenshot stored on a server | Screenshots analyzed locally, rarely uploaded |
| One-size-fits-all rules            | Role-specific: marketing can be on Facebook, accounting cannot |
| Heavy server infrastructure needed | Lightweight — the employee's computer does the thinking |
| Summary-only data, no drill-down  | Full metadata + summary — drill from dashboard to individual events |

---

### Who Benefits

- **Business Owners**: Get real productivity insights without massive server bills.
- **HR Teams**: See clear dashboards with employee explanations, not just raw data.
- **Employees**: Get fair, role-based treatment and the chance to explain themselves.
- **IT Teams**: Deploy a lightweight agent that doesn't kill laptop performance or bandwidth.

---

---

## Part 2: Architecture (Technical)

### Tech Stack

| Layer              | Technology                                | Rationale                                                                                       |
| ------------------ | ----------------------------------------- | ----------------------------------------------------------------------------------------------- |
| **Endpoint Agent** | Rust (workspace with 9 crates)            | Zero-cost abstractions, minimal runtime overhead, cross-platform (Windows/macOS/Linux), no GC pauses for real-time popup responsiveness. |
| **Backend API**    | Go (net/http + chi or gin)                | High-concurrency HTTP server, excellent WebSocket support, fast compilation, strong standard library. Single binary deployment. |
| **Admin Portal**   | Next.js 14+ (App Router, TypeScript)      | Server components for dashboard performance, real-time WebSocket updates for fleet monitoring, React for interactive UI. |
| **Agent Local DB** | SQLite (via rusqlite, encrypted with SQLCipher) | Embedded, zero-config, fast local storage. No server dependency. 7-day ring buffer.            |
| **Server DB**      | PostgreSQL 16                             | ACID compliance for configuration, employees, devices, commands. JSONB for flexible policy storage. |
| **Analytics DB**   | ClickHouse                                | Columnar storage for high-throughput time-series inserts. Billions of rows, sub-second queries. |
| **Object Storage** | MinIO (S3-compatible)                     | Screenshot images, ML model binaries, agent installer packages. Self-hosted, no cloud vendor lock-in. |
| **Cache/Sessions** | Redis 7                                   | Session management, rate limiting, WebSocket connection registry, fleet status cache.           |
| **Auth**           | Keycloak (OIDC/SAML)                      | Enterprise SSO integration. Manages admin portal users. Agent auth is mTLS, not OIDC.          |
| **Containerization** | Docker + Docker Compose (dev)           | Consistent dev environment. Production: Kubernetes or bare-metal with systemd.                  |
| **CI/CD**          | GitHub Actions                            | Build agent for all 3 platforms, run Go tests, lint Next.js, deploy.                            |

---

### Design Philosophy

**Edge-first, server-light.** The endpoint agent is the intelligence layer. The server is persistence and configuration. This inverts the traditional monitoring architecture where the server processes everything and the endpoint is a dumb collector.

Three principles drive every decision:

1. **Classify locally, upload everything.** Raw data stays on the device. The agent sends full metadata (app name, window title, process info, duration, classification, confidence) plus an aggregated summary. The server receives both — the summary for dashboards, the metadata for drill-down analysis and compliance auditing.
2. **React in real time.** Popups fire within milliseconds of a rule violation, not after a server round-trip.
3. **Server manages policy, not processing.** The server defines rules and stores results. It does not classify behavior.

---

### System Overview

```
┌──────────────────────────────────────────────────────┐
│                    ENDPOINT AGENT                      │
│                                                       │
│  ┌─────────────┐  ┌─────────────┐  ┌──────────────┐  │
│  │ Collectors   │  │ Local Rule   │  │  Local ML    │  │
│  │ (active win, │──▶│ Engine       │──▶│  Classifier  │  │
│  │  idle, proc) │  │ (role rules) │  │  (ONNX)      │  │
│  └─────────────┘  └──────┬───────┘  └──────────────┘  │
│                          │                             │
│                    Violation?                           │
│                    YES ↓                               │
│              ┌───────────────────┐                      │
│              │  Popup Manager    │                      │
│              │  (toast / explain │                      │
│              │   / soft-block)   │                      │
│              └────────┬──────────┘                      │
│                       │                                 │
│              ┌────────▼──────────┐                      │
│              │  Local Store      │                      │
│              │  (SQLite, 7-day)  │                      │
│              │  ├─ priority      │──▶ immediate upload  │
│              │  └─ bulk          │──▶ every 5h upload   │
│              └────────┬──────────┘                      │
│                       │                                 │
│       ┌───────────────┼───────────────┐                 │
│  ┌────▼─────┐  ┌──────▼──────┐  ┌────▼──────────┐     │
│  │ Watchdog  │  │ Screenshot   │  │  Self-Heal    │     │
│  │ Process   │  │ Commander    │  │  Service       │     │
│  │ (monitors │  │ (on-demand   │  │  (unkillable   │     │
│  │  agent)   │  │  capture)    │  │   systemd/svc) │     │
│  └──────────┘  └─────────────┘  └───────────────┘     │
└──────────────────────────┬───────────────────────────────┘
                           │
                     HTTPS + mTLS
                           │
                           ▼
┌──────────────────────────────────────────────────────┐
│                    API GATEWAY                         │
│                                                       │
│  POST /v1/events/bulk       — accept batch summaries               │
│  POST /v1/events/priority   — accept immediate alerts              │
│  POST /v1/events/popup      — accept popup explanations            │
│  GET  /v1/rules/sync        — agent pulls role rules                │
│  GET  /v1/models/latest     — agent pulls ML models                 │
│  POST /v1/enroll            — device enrollment (Employee ID)      │
│  POST /v1/screenshot/request — admin requests on-demand screenshot │
│  POST /v1/screenshot/upload  — agent uploads on-demand screenshot   │
│  GET  /v1/devices/status    — admin checks device fleet status      │
│  POST /v1/companies         — company registration                  │
│  POST /v1/employees         — employee registration                 │
│  GET  /v1/employees/:id     — lookup employee by ID                 │
│  WS   /v1/commands          — push commands to agent (screenshot,   │
│                               policy update, uninstall)             │
│                                                                       │
│  Direct writes (no message broker):                    │
│       ├── PostgreSQL  (rules, commands, explanations)  │
│       └── ClickHouse  (rollups, analytics summaries)   │
└──────────────────────────┬───────────────────────────────┘
                           │
                           ▼
┌──────────────────────────────────────────────────────┐
│                   ADMIN PORTAL                         │
│                                                       │
│  /admin/app-rules   — define role-based classifications│
│  /admin/alerts      — define alert rules & thresholds  │
│  /devices           — manage enrolled devices          │
│  /employees         — manage employees & roles          │
│  /reports           — view productivity dashboards      │
│  /me                — employee's own data page         │
└──────────────────────────────────────────────────────┘
```

---

### Component Details

#### Endpoint Agent

Nine crates in the Rust workspace:

| Crate               | Responsibility                                                    |
| ------------------- | ----------------------------------------------------------------- |
| `agent-core`        | Main process. Orchestrates collectors, rule engine, popup manager. |
| `agent-collectors`  | Active window, idle state, process enumeration.                   |
| `agent-tray`        | System tray process. Renders popups (toast, modal with explanation, soft-block). |
| `agent-ml`          | ONNX Runtime inference. Two models: app title classifier and screenshot classifier. |
| `agent-store`       | Encrypted SQLite ring buffer. 7-day local retention. Two queues: priority and bulk. |
| `agent-uploader`    | Dual-channel uploader. Priority channel sends immediately. Bulk channel batches every N hours (configurable per policy). |
| `agent-watchdog`    | Monitors and restarts other agent processes. If watchdog is killed, agent-core restarts it. If agent-core is killed, watchdog restarts it. Mutual protection — killing one triggers the other to respawn it. |
| `agent-service`     | Installs and manages the system-level service (systemd on Linux, LaunchDaemon on macOS, Windows Service on Windows). Configures `Restart=always` / auto-restart policies. Protects binary and config files from deletion. Detects tampering attempts (process kill, service stop, file modification) and sends tamper events on priority channel. |
| `agent-screenshot`  | Handles on-demand screenshot requests from admin. Listens for commands via WebSocket (`/v1/commands`). On receiving a capture command, takes a screenshot, classifies it locally, and uploads the image or metadata to `POST /v1/screenshot/upload`. |

**Local Rule Engine** (inside `agent-core`):

- On enrollment, agent fetches its role's rules: app classifications (productive / unproductive / neutral) and alert rules (time thresholds, popup types).
- Rules are refreshed every 5 minutes via `GET /v1/rules/sync`.
- On every window focus change, the rule engine evaluates: is this app classified as unproductive for this role?
- If yes → immediately triggers popup. No server round-trip.

**Local ML Classifier** (`agent-ml`):

- App/Title classifier (~5MB ONNX model): given `(app_name, window_title)`, outputs a productivity class and confidence score.
- Screenshot classifier (~20MB ONNX model): given a screenshot frame, outputs a content category and confidence score.
- Both models are downloaded from `GET /v1/models/latest` and cached locally.
- The ML classifier supplements the rule engine — it handles apps not in the rule database.

**Classification Priority**:
1. Exact match in rule database → use that classification immediately.
2. No match → run ML classifier → use its output if confidence > threshold.
3. Low confidence → classify as "unknown" and include in next bulk upload for admin review.

**Popup Manager** (inside `agent-tray`):

Three popup types:

| Type            | Behavior                                                              |
| --------------- | --------------------------------------------------------------------- |
| Toast           | Informational, auto-dismisses after 10 seconds. Low-severity nudge.   |
| Modal + Explain | Requires user to type an explanation before dismissing.               |
| Soft-Block      | Blocks the flagged app. Requires admin override or employee explanation. |

Explanations are captured as `PopupEvent` messages with a `explanation` field and sent on the priority upload channel immediately.

#### Cross-Platform Agent Details

The agent must run identically on **Windows**, **macOS**, and **Linux**. Here is how each platform-specific concern is handled:

**Active Window & Process Collection** (`agent-collectors`):

| Concern                | Windows                          | macOS                              | Linux                              |
| ---------------------- | -------------------------------- | ---------------------------------- | ---------------------------------- |
| Active window title    | `GetForegroundWindow` + `GetWindowTextW` (Win32 API) | `NSWorkspace.shared.frontmostApplication` via `core-foundation` FFI | `XGetInputFocus` + `XFetchName` (X11) / `wndctl` (Wayland) |
| Process name & PID     | `CreateToolhelp32Snapshot` + `Process32First/Next` | `proc_listallpids` + `proc_pidpath` via `libproc` | `/proc/[pid]/comm` + `/proc/[pid]/cmdline` |
| Idle detection         | `GetLastInputInfo` (Win32)        | `CGEventSourceSecondsSinceLastEventType` (CoreGraphics) | `XScreenSaverQueryInfo` (X11) / `org.freedesktop.ScreenSaver` (Wayland) |
| Screenshot capture     | `BitBlt` from desktop DC (GDI)    | `CGWindowListCreateImage` (CoreGraphics) | `XGetImage` (X11) / `xdg-screenshot` portal (Wayland) |
| System tray            | Windows API `Shell_NotifyIconW`   | `NSStatusItem` via `cocoa` FFI     | `StatusNotifierItem` via D-Bus    |

**Unkillable Service** (`agent-service`):

| Platform  | Mechanism                                                                                                   | Restart Policy                                                    | Protection                                                                                                   |
| --------- | ----------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| Windows   | Windows Service (via `windows-service` crate) registered as `AINMS Agent`. Runs as `LocalSystem`.           | SCM recovery: `SC_ACTION_RESTART` with 0-second delay, no limit. | Deny `SERVICE_STOP` to non-admin. File permissions on binary/config set to admin-only. Tamper-log on stop.   |
| macOS     | LaunchDaemon plist in `/Library/LaunchDaemons/com.ainms.agent.plist`. Runs as root.                         | `KeepAlive=true`, `RunAtLoad=true`.                               | SIP-protected paths for binary. `chflags schg` on config. Watchdog respawns if killed. Unload requires admin. |
| Linux     | systemd unit file in `/etc/systemd/system/ainms-agent.service`. Runs as `root` or dedicated `ainms` user.  | `Restart=always`, `RestartSec=3`, `StartLimitIntervalSec=0`.      | `ProtectSystem=strict`, `ProtectHome=read-only`, binary `chattr +i`. Unload requires root. Tamper-log on stop. |

**Agent Installer** (per platform):

| Platform | Format | Install Flow                                                                                         |
| -------- | ------ | ---------------------------------------------------------------------------------------------------- |
| Windows  | `.msi` | MSI package via WiX Toolset. Installs service, registers mTLS cert, writes config. Prompts for Employee ID. |
| macOS    | `.dmg` | DMG with `.pkg` installer. Installs LaunchDaemon, registers mTLS cert, writes config. Prompts for Employee ID. Terminal popup for Employee ID if headless. |
| Linux    | `.deb` / `.rpm` | Package installs systemd unit, config, binary. Postinst script prompts for Employee ID via terminal.  |

**Build & Cross-Compilation**:

- Agent workspace uses Cargo with a `Makefile.toml` (via `cargo-make`) for cross-compilation targets: `x86_64-pc-windows-msvc`, `aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`.
- Platform-specific code isolated behind `cfg(target_os)` attributes with per-OS modules (`src/os/windows.rs`, `src/os/macos.rs`, `src/os/linux.rs`).
- CI builds all targets via GitHub Actions matrix, produces installer packages, and uploads to MinIO for distribution via `GET /v1/agents/latest?platform={windows|macos|linux}`.

#### API Gateway

Single Go service (using `chi` router, `pgx` for PostgreSQL, `clickhouse-go` for ClickHouse, `nhooyr.io/websocket` for WebSocket command channel) that handles all agent and portal communication:

- **Authentication**: mTLS for agent connections (client certs issued during enrollment, validated against a CA stored in PostgreSQL). OIDC/SAML via Keycloak for admin portal.
- **Batch upload endpoint**: Accepts both per-event metadata AND aggregated summaries from agents. Each batch contains a `metadata` array (individual events with full detail) and a `summary` object (aggregated stats). Writes metadata to ClickHouse `app_usage_events`, summaries to ClickHouse `app_usage_summary`.
- **Priority event endpoint**: Accepts alerts, explanations, tamper events. Writes to PostgreSQL + ClickHouse immediately.
- **Rule sync endpoint**: Returns role-specific rules for the requesting device's employee.
- **Model serving endpoint**: Returns the latest ML model binary and version checksum.
- **Enrollment endpoint**: Registers new devices and assigns them to employees.

No Kafka, no separate enricher. The Gateway writes directly to databases because:

- Volume is ~300x lower than the old raw-event stream (summaries vs. per-second events).
- Data is pre-classified by the agent — no server-side processing needed.
- Batch inserts into ClickHouse are fast enough for the expected load.

#### Data Stores

**PostgreSQL** — configuration and transactional data:

| Table                    | Purpose                                           |
| ------------------------ | ------------------------------------------------- |
| `tenants`                | Multi-tenant organizations                        |
| `companies`              | Company registration (name, plan, settings)       |
| `users` / `teams`        | Admin users and team structure                    |
| `employees`              | Employee profiles with role assignment and unique Employee ID |
| `devices`                | Enrolled endpoint devices, linked to employees    |
| `device_sessions`        | Live connection tracking — device_id, connected_at, disconnected_at, status (online/offline/pending) |
| `policies`               | Monitoring policies per tenant (upload interval, screenshot settings, etc.) |
| `app_classifications`    | App → productivity class mapping per role         |
| `alert_rules`            | Threshold rules per role                          |
| `alerts_fired`           | Historical record of every popup that was shown   |
| `pending_commands`       | Commands pushed to agents via WebSocket (policy updates, screenshot requests, uninstall, etc.) |
| `popup_explanations`     | Employee explanations for flagged app usage       |
| `screenshot_requests`    | On-demand screenshot requests — who requested, which device, reason, timestamp, status (pending/completed/expired) |
| `employee_registrations` | Pre-registration queue — employee records created before device enrollment, with status (active/pending/deactivated) |

**ClickHouse** — analytics and time-series data:

| Table                | TTL    | Purpose                                        |
| -------------------- | ------ | ---------------------------------------------- |
| `app_usage_summary`  | 24 mo  | Aggregated app usage stats from agent batches (rollup per app per employee per day) |
| `app_usage_events`   | 12 mo  | Every individual app usage event with full metadata: app_name, window_title, process_name, process_id, start_time, end_time, duration_sec, classification, confidence, role_id, device_id |
| `idle_sessions`      | 24 mo  | Idle/active session boundaries                  |
| `popup_events`       | 12 mo  | Every popup shown, with explanation and full app context |
| `alert_fired_log`    | 24 mo  | Alert rule evaluations and their outcomes       |
| `agent_health`       | 90 d   | Agent heartbeat and health telemetry           |
| `screenshot_metadata`| 30 d   | Screenshot classification results (not images) |
| `network_flow_summary`| 90 d  | Aggregated network activity                    |
| `screenshot_ondemand`| 90 d   | On-demand screenshot captures (who requested, device, classification, upload URL, reason provided) |
| `device_fleet_status` | 90 d  | Device connection events — connect, disconnect, heartbeat misses, tamper detections |
| `tamper_events`      | 24 mo  | Every tamper attempt (process kill, service stop, file modification, uninstall attempt) |

Note: Only `screenshot_metadata` is stored — not the images themselves. Images stay on the endpoint unless policy explicitly requires upload to MinIO. The `app_usage_events` table stores full metadata for every usage session, enabling drill-down from the summary dashboard to individual event detail.

#### Admin Portal

Next.js 14+ application (App Router, TypeScript, Tailwind CSS) with server components for data-heavy pages and client components for interactive dashboards.

**Tech details**:
- Server Components for dashboard data fetching (direct ClickHouse/PostgreSQL queries via Go API)
- Client Components with real-time WebSocket updates for fleet monitoring and alerts
- shadcn/ui component library for consistent UI
- Recharts for analytics visualizations
- Uses Server Actions for mutations (company registration, employee creation, screenshot requests)
- OIDC authentication via Keycloak for admin users

Key pages:

- **Dashboard**: Real-time productivity overview across teams and departments.
- **App Rules**: Define which apps are productive/unproductive per role.
- **Alert Rules**: Set thresholds (e.g., "more than 30 min/day on entertainment = popup").
- **Employees**: Manage employee profiles and role assignments.
- **Devices**: Monitor enrolled devices, push policy updates, send commands. Live fleet view showing online/offline/pending status, last-seen timestamps, and devices that haven't reported in.
- **Screenshots**: On-demand screenshot capture. Select any enrolled device, request a screenshot, and view results (with classification) within seconds. Every request is logged with requester identity and reason.
- **Companies**: Register and manage companies (multi-tenant). Each company has its own policies, employees, and device fleet.
- **Employee Registration**: Register employees before device enrollment. Each employee gets a unique Employee ID. Only registered employees can enroll devices.
- **Reports**: Historical analytics, trend charts, export capabilities.
- **Alerts Feed**: Live and historical alerts, including employee explanations.
- **My Data** (employee view): Each employee can see their own productivity stats.

---

### Data Flows

#### Normal Operation: App Usage Tracking

```
1. Employee opens Chrome → active_window collector detects focus change
2. Local rule engine checks: Chrome → "browser" → role allows browsers → classified as neutral/productive
3. Usage recorded in local SQLite store with FULL METADATA: {app_name, window_title, process_name, process_id, start_time, end_time, duration_sec, classification, confidence, role_id, device_id}
4. Every 5 hours (configurable): agent sends batch to POST /v1/events/bulk containing:
   a. summary: aggregated stats per app (total_duration, session_count, classification_breakdown)
   b. metadata: every individual usage event with full detail as listed in step 3
5. Gateway writes summary to ClickHouse app_usage_summary, metadata to ClickHouse app_usage_events
6. Portal queries ClickHouse → shows both summary dashboards AND drill-down per-event detail
```

#### Violation: Unproductive App

```
1. Employee opens Netflix → active_window collector detects focus change
2. Local rule engine: Netflix → "entertainment" → role does not allow → VIOLATION
3. Popup manager shows modal immediately: "Netflix is not a productive app for your role. Please explain."
4. Employee types explanation: "Watching training video for project research"
5. Explanation sent on priority channel with FULL METADATA: POST /v1/events/popup → {explanation, app_name, window_title, process_name, duration, classification, confidence, timestamp} → PostgreSQL popup_explanations + ClickHouse popup_events
6. Usage recorded locally with full detail, bulk queue
7. HR sees the incident + explanation + full context in the alerts feed on the portal
```

#### Violation: Unknown App (ML Classification)

```
1. Employee opens "Figma" → active_window collector detects focus change
2. Local rule engine: Figma → not in rule database → UNKNOWN
3. Local ML classifier: "Figma" + "design tool" → classified as "productive" (confidence: 0.94)
4. Confidence above threshold → accepted as productive, no popup
5. Full metadata + classification result included in next bulk upload for admin review: {app_name: "Figma", window_title: "design tool", process_name, ml_classification: "productive", ml_confidence: 0.94, rule_classification: null}
6. Admin reviews "unknown app" list with full metadata context, adds Figma to rule database for designers
```

#### Screenshot Classification

```
1. Agent captures screenshot (interval per policy)
2. Local ML classifier processes image → output: {category: "entertainment", confidence: 0.87}
3. IF policy requires upload AND category is flagged → upload image to MinIO
4. IF policy does not require upload → send only metadata to server
5. Metadata stored in ClickHouse screenshot_metadata: {device_id, ts, category, confidence, uploaded: bool}
```

#### Enrollment (Employee ID-Based)

```
1. Admin registers company in the portal → gets company_id
2. HR creates employee record in portal → gets Employee ID (e.g., EMP-0042)
3. Agent installer runs on employee device, prompts for Employee ID
4. Agent calls POST /v1/enroll with {employee_id: "EMP-0042", device_info: {...}}
5. Server looks up employee by ID:
   a. Employee exists and active → issue device certificate, assign role & rules
   b. Employee not found → reject enrollment, show error
   c. Employee deactivated → reject enrollment, show error
6. Agent receives: device_id, employee profile, role, initial rule set, mTLS certificate
7. Agent downloads ML models via GET /v1/models/latest
8. Agent opens WebSocket connection to /v1/commands for real-time command channel
9. Agent registers as system service (unkillable) via agent-service
10. Agent begins collecting, classifying, and storing locally
11. First bulk upload begins after the configured interval
12. Server records device in device_sessions as "online"
```

#### On-Demand Screenshot

```
1. HR/Admin clicks "Take Screenshot" on a device in the portal
2. Portal sends POST /v1/screenshot/request with {device_id, requested_by, reason}
3. Server stores request in screenshot_requests table
4. Server pushes command to agent via WebSocket /v1/commands: {type: "screenshot_capture", request_id, policy: "upload_image" | "metadata_only"}
5. Agent's agent-screenshot receives command, captures screen immediately
6. Agent-ml classifies screenshot locally → {category, confidence}
7. Based on policy in command:
   a. "upload_image" → agent uploads full screenshot image to MinIO via POST /v1/screenshot/upload
   b. "metadata_only" → agent sends only classification metadata
8. Server stores result in ClickHouse screenshot_ondemand + updates screenshot_requests status to "completed"
9. HR/Admin sees the screenshot (or metadata) on the portal within seconds
10. If agent is offline → request queued in pending_commands, delivered when agent reconnects
```

#### Device Fleet Monitoring

```
1. Agent connects to server → device_sessions records {device_id, status: "online", connected_at}
2. Agent sends heartbeat every 60 seconds (configurable)
3. Server tracks last heartbeat per device
4. If heartbeat missed for N minutes (configurable) → device marked "potentially_offline"
5. Agent disconnects → device_sessions updates {status: "offline", disconnected_at}
6. Tamper detected (process killed, service stopped, file modified):
   a. Watchdog restarts agent immediately
   b. Agent sends priority event with tamper details
   c. Server records in ClickHouse tamper_events
   d. Admin portal flags device in fleet view
7. Admin portal fleet dashboard shows:
   - Total enrolled devices
   - Currently online (green)
   - Potentially offline / missed heartbeat (yellow)
   - Tampered / offline for extended period (red)
   - Employees with no enrolled device yet (pending)
```

---

### Deployment

#### Development (Docker Compose)

| Service      | Port | Purpose               |
| ------------ | ---- | --------------------- |
| PostgreSQL   | 5440 | Config + transactional |
| ClickHouse   | 8123 | Analytics             |
| Redis        | 6390 | Sessions + cache       |
| MinIO        | 9101 | Screenshot storage    |
| Gateway+API  | 8440 | Single backend service|
| Portal       | 3440 | Admin dashboard       |
| Keycloak     | 8443 | Auth (optional)       |

#### Production Considerations

- **Gateway+API** can be deployed as a single Go binary behind a load balancer. For MVP, no need to separate them.
- **ClickHouse** handles analytics at scale. Start with a single node; add replicas if needed.
- **Agent** is distributed as an MSI (Windows), DMG (macOS), or deb/rpm (Linux) installer.
- **ML models** are versioned and served from the Gateway. Agent checks for updates every hour.
- **No Kafka, no Redpanda** needed. The architecture removes this dependency entirely.

---

### Security Model

- **mTLS**: Agent ↔ Gateway communication uses mutual TLS. Each device has a client certificate issued during enrollment.
- **Encryption at rest**: Local SQLite store is encrypted. Agent data is only readable by the agent process.
- **Role separation**: Employees see only their own data (`/me`). HR sees team data. Admins see everything.
- **GDPR compliance**: Right to erasure is supported. PII catalog tracked in `pii_catalog.yaml`. Screenshot data stays on-device by default.
- **Tamper resistance**: Agent watchdog + agent-core in mutual protection. If either is killed, the other restarts it. Agent-service prevents service stop by non-admin users. Tamper events are sent on the priority channel immediately. Agent binary and config files are protected from modification or deletion.
- **Unkillable service**: Agent installs as a system-level service that auto-restarts on any termination. On Linux: systemd with `Restart=always`, `RestartSec=3`. On macOS: LaunchDaemon with `KeepAlive=true`. On Windows: Windows Service with recovery options (restart on failure, 0-second delay). The only way to uninstall is a cryptographically signed uninstall command from the admin portal.
- **On-demand screenshot audit**: Every remote screenshot request is logged with who requested it, when, the reason they provided, and the device targeted. This creates an accountability trail preventing abuse.
- **Employee ID verification**: No device can enroll without a valid, active Employee ID. This prevents anonymous or unauthorized devices from joining the fleet.

---

### What Was Removed From the Previous Architecture

| Component       | Previous Role                         | Why Removed                                     |
| --------------- | ------------------------------------- | ----------------------------------------------- |
| Kafka/Redpanda  | Event bus between Gateway & Enricher | Agent sends pre-classified summaries, not raw events. Volume too low to justify a message broker. |
| Enricher service | Classified apps, evaluated alerts    | All classification and alerting moved to the agent. Remaining rollup writes absorbed by Gateway. |
| Raw events stream | Gateway → Kafka → Enricher         | Replaced by two channels: priority (immediate) and bulk (periodic). |

---

### Implementation Priority

| Priority | Component                                   | Effort   | Rationale                                          |
| -------- | ------------------------------------------- | -------- | -------------------------------------------------- |
| P0       | Company registration + Employee ID enrollment | Medium | Must exist before any device can connect. Foundational. |
| P0       | Unkillable agent service (`agent-service`)   | Medium   | Agent must survive tampering. Core reliability requirement. |
| P0       | Cross-platform collectors (`agent-collectors`)| High    | Must work on Windows, macOS, Linux. Platform-specific code for active window, idle, process, screenshot. |
| P0       | Popup with explanation (two-way)            | Medium   | Core product differentiator—"they must explain."    |
| P0       | Agent rule sync + local rule engine          | Medium   | Enables real-time popups without server delay.      |
| P1       | Device fleet monitoring + live status        | Medium   | Admins need to see which devices are connected.     |
| P1       | On-demand screenshot capture (`agent-screenshot`) | Medium | HR/Admin real-time visibility into any device. |
| P1       | Batch upload with metadata + summary (`agent-uploader`) | Medium | Server receives both per-event detail and aggregated rolls. |
| P1       | WebSocket command channel (`/v1/commands`)   | Medium   | Required for on-demand screenshots and real-time commands. |
| P1       | Remove Kafka/Enricher, merge into Gateway    | Low      | Simplify infrastructure before building on top.    |
| P2       | Local ML classifier crate (`agent-ml`)      | High     | Handles unknown apps without round-trip.            |
| P2       | Screenshot capture + local classification    | High     | Privacy-preserving, saves bandwidth.               |
| P3       | Demote server-side alert evaluator to fallback | Low    | Keep it for retroactive analysis, not primary path. |
