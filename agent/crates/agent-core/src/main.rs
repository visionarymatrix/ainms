mod config;
mod dialog;
pub(crate) mod os;
mod rule_engine;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use chrono::Utc;
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio::time::{interval, sleep};
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

use agent_proto::events::{
    AppUsageEventMeta, AppUsageSummary, BulkEventRequest, EnrollmentRequest, EnrollmentResponse,
};

const MAX_EVENT_BUFFER: usize = 10_000;
const LOGIN_MAX_RETRIES: u32 = 5;
const LOGIN_BASE_DELAY_SECS: u64 = 1;
const ENROLL_MAX_RETRIES: u32 = 5;
const ENROLL_BASE_DELAY_SECS: u64 = 2;
const CONSECUTIVE_HB_FAILURES_FOR_REENROLL: u32 = 3;
const UPLOAD_RETRY_DELAY_SECS: u64 = 5;
const IDLE_THRESHOLD_SECS: f64 = 300.0;
const SCREENSHOT_INTERVAL_SECS: u64 = 300;

// ── CLI / Config ────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
enum ServiceCommand {
    #[command(about = "Install the agent as a system service (requires admin/root)")]
    Install,
    #[command(about = "Uninstall the agent system service (requires admin/root)")]
    Uninstall,
    #[command(about = "Start the agent system service")]
    Start,
    #[command(about = "Stop the agent system service")]
    Stop,
}

#[derive(Parser, Debug)]
#[command(name = "ainms-agent", version = "0.2.0", about = "AINMS Agent")]
struct Args {
    #[arg(long, value_name = "ID")]
    employee_id: Option<String>,

    #[arg(long)]
    company_id: Option<String>,

    #[arg(long)]
    server: Option<String>,

    #[arg(long)]
    auth_email: Option<String>,

    #[arg(long)]
    auth_password: Option<String>,

    #[arg(long)]
    config: Option<String>,

    #[arg(long, hide = true)]
    run_as_service: bool,

    #[command(subcommand)]
    service: Option<ServiceCommand>,
}

#[derive(Debug, Deserialize)]
struct ConfigFile {
    employee_id: Option<String>,
    company_id: Option<String>,
    server: Option<String>,
    auth_email: Option<String>,
    auth_password: Option<String>,
}

fn resolve_config(args: &Args) -> ResolvedConfig {
    let file_cfg = match &args.config {
        Some(path) => {
            let contents = std::fs::read_to_string(path).unwrap_or_else(|e| {
                warn!("Failed to read config file {}: {}", path, e);
                String::new()
            });
            toml::from_str(&contents).unwrap_or_else(|e| {
                warn!("Failed to parse config file: {}", e);
                ConfigFile {
                    employee_id: None,
                    company_id: None,
                    server: None,
                    auth_email: None,
                    auth_password: None,
                }
            })
        }
        None => ConfigFile {
            employee_id: None,
            company_id: None,
            server: None,
            auth_email: None,
            auth_password: None,
        },
    };

    let employee_id = args
        .employee_id
        .clone()
        .or(file_cfg.employee_id)
        .or_else(|| std::env::var("AINMS_EMPLOYEE_ID").ok())
        .unwrap_or_else(|| "unknown".to_string());

    let company_id = args
        .company_id
        .clone()
        .or(file_cfg.company_id)
        .or_else(|| std::env::var("AINMS_COMPANY_ID").ok())
        .unwrap_or_else(|| "unknown".to_string());

    let server = args
        .server
        .clone()
        .or(file_cfg.server)
        .or_else(|| std::env::var("AINMS_SERVER").ok())
        .unwrap_or_else(|| "http://173.249.47.143:8440".to_string());

    let auth_email = args
        .auth_email
        .clone()
        .or(file_cfg.auth_email)
        .or_else(|| std::env::var("AINMS_EMAIL").ok())
        .unwrap_or_else(|| "superadmin@ainms.io".to_string());

    let auth_password = args
        .auth_password
        .clone()
        .or(file_cfg.auth_password)
        .or_else(|| std::env::var("AINMS_PASSWORD").ok())
        .unwrap_or_else(|| "changeme".to_string());

    ResolvedConfig {
        employee_id,
        company_id,
        server,
        auth_email,
        auth_password,
    }
}

struct ResolvedConfig {
    employee_id: String,
    company_id: String,
    server: String,
    auth_email: String,
    auth_password: String,
}

// ── API types ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct LoginResponse {
    token: String,
}

struct ActiveWindowSession {
    app_name: String,
    window_title: String,
    process_name: String,
    process_id: i32,
    start_time: chrono::DateTime<Utc>,
}

struct AgentState {
    device_id: String,
    device_token: String,
    jwt_token: String,
    events: Vec<AppUsageEventMeta>,
    consecutive_heartbeat_failures: u32,
    active_window: Option<ActiveWindowSession>,
    idle_since: Option<chrono::DateTime<Utc>>,
}

impl AgentState {
    fn push_events(&mut self, new_events: Vec<AppUsageEventMeta>) {
        let overflow = self.events.len() + new_events.len();
        if overflow > MAX_EVENT_BUFFER {
            let to_drop = overflow.saturating_sub(MAX_EVENT_BUFFER);
            warn!(
                to_drop,
                "Event buffer full ({}), dropping {} oldest events",
                MAX_EVENT_BUFFER, to_drop
            );
            self.events.drain(..to_drop.min(self.events.len()));
        }
        self.events.extend(new_events);
    }

    fn requeue_events(&mut self, events: Vec<AppUsageEventMeta>) {
        let overflow = self.events.len() + events.len();
        if overflow > MAX_EVENT_BUFFER {
            let available = MAX_EVENT_BUFFER.saturating_sub(self.events.len());
            let events: Vec<_> = events
                .into_iter()
                .rev()
                .take(available)
                .collect();
            self.events.extend(events);
        } else {
            self.events.extend(events);
        }
    }

    fn close_active_window(&mut self) -> Option<AppUsageEventMeta> {
        let session = self.active_window.take()?;
        let now = Utc::now();
        let duration = (now - session.start_time).num_seconds() as f64;
        let (classification, confidence) = classify_process(&session.process_name);
        let event = AppUsageEventMeta {
            app_name: session.app_name.clone(),
            window_title: session.window_title.clone(),
            process_name: session.process_name.clone(),
            process_id: session.process_id,
            start_time: session.start_time,
            end_time: now,
            duration_sec: duration,
            classification,
            confidence,
            role_id: None,
            device_id: self.device_id.clone(),
        };
        Some(event)
    }
}

// ── Exponential backoff helper ───────────────────────────────────────────────

async fn retry_with_backoff<F, Fut, T>(
    label: &str,
    max_retries: u32,
    base_delay_secs: u64,
    op: F,
) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut attempt = 0u32;
    loop {
        match op().await {
            Ok(val) => return Ok(val),
            Err(e) => {
                if attempt >= max_retries {
                    error!(attempt, max_retries, "All retries exhausted for {}", label);
                    return Err(e);
                }
                let delay_secs = base_delay_secs * 2u64.pow(attempt);
                warn!(
                    attempt,
                    delay_secs,
                    "Retrying {} after error: {}", label, e
                );
                sleep(Duration::from_secs(delay_secs)).await;
                attempt += 1;
            }
        }
    }
}

// ── API functions ────────────────────────────────────────────────────────────

async fn login(
    client: &reqwest::Client,
    server: &str,
    email: &str,
    password: &str,
) -> Result<String> {
    let req = LoginRequest {
        email: email.to_string(),
        password: password.to_string(),
    };
    let resp = client
        .post(format!("{}/v1/auth/login", server))
        .json(&req)
        .send()
        .await
        .context("Failed to send login request")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Login failed with status {}: {}", status, body);
    }

    let login_resp: LoginResponse = resp.json().await.context("Failed to parse login response")?;
    Ok(login_resp.token)
}

async fn login_with_retry(
    client: &reqwest::Client,
    server: &str,
    email: &str,
    password: &str,
) -> Result<String> {
    retry_with_backoff("login", LOGIN_MAX_RETRIES, LOGIN_BASE_DELAY_SECS, || {
        login(client, server, email, password)
    })
    .await
}

async fn enroll(
    client: &reqwest::Client,
    server: &str,
    employee_id: &str,
    company_id: &str,
    jwt_token: &str,
) -> Result<EnrollmentResponse> {
    let hostname = gethostname::gethostname()
        .into_string()
        .unwrap_or_else(|_| "unknown".to_string());

    let os_type = os::os_type();
    let os_version = os::os_version();
    let fingerprint = os::generate_fingerprint();
    let cpu_info = os::cpu_info();
    let ram_info = os::ram_info();
    let disk_info = os::disk_info();
    let mac_addresses = os::mac_addresses();
    let ip_addresses = os::ip_addresses();

    let req = EnrollmentRequest {
        employee_id: employee_id.to_string(),
        company_id: company_id.to_string(),
        hostname: hostname.clone(),
        os_type: os_type.clone(),
        os_version: os_version.clone(),
        fingerprint: fingerprint.clone(),
        cpu_info: if cpu_info.is_empty() { None } else { Some(cpu_info) },
        ram_info: if ram_info.is_empty() { None } else { Some(ram_info) },
        disk_info: if disk_info.is_empty() { None } else { Some(disk_info) },
        mac_addresses: if mac_addresses.is_empty() { None } else { Some(mac_addresses) },
        ip_addresses: if ip_addresses.is_empty() { None } else { Some(ip_addresses) },
    };

    info!(
        employee_id,
        company_id,
        hostname,
        os_type,
        os_version,
        fingerprint = &fingerprint[..16.min(fingerprint.len())],
        "Enrolling device"
    );

    let resp = client
        .post(format!("{}/v1/enroll", server))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .json(&req)
        .send()
        .await
        .context("Failed to send enrollment request")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Enrollment failed with status {}: {}", status, body);
    }

    let enroll_resp: EnrollmentResponse = resp
        .json()
        .await
        .context("Failed to parse enrollment response")?;

    info!(device_id = %enroll_resp.device_id, status = %enroll_resp.status, "Enrolled");

    Ok(enroll_resp)
}

async fn enroll_with_retry(
    client: &reqwest::Client,
    server: &str,
    employee_id: &str,
    company_id: &str,
    jwt_token: &str,
) -> Result<EnrollmentResponse> {
    retry_with_backoff(
        "enrollment",
        ENROLL_MAX_RETRIES,
        ENROLL_BASE_DELAY_SECS,
        || enroll(client, server, employee_id, company_id, jwt_token),
    )
    .await
}

fn classify_process(name: &str) -> (String, f64) {
    let lower = name.to_lowercase();
    let productive = [
        "ssh", "bash", "zsh", "vim", "nano", "code", "cargo", "go", "python",
        "node", "rustc", "rust-analyzer", "git", "make", "cmake", "docker",
        "kubectl", "terraform", "ansible", "java", "javac", "mvn", "gradle",
        "powershell", "cmd", "devenv", "msbuild", "dotnet",
    ];
    let neutral = [
        "firefox", "chrome", "chromium", "safari", "edge", "brave",
        "teams", "slack", "discord", "zoom", "thunderbird", "mail",
        "outlook", "explorer", "iexplore",
    ];

    for p in &productive {
        if lower.contains(p) {
            return ("productive".to_string(), 0.85);
        }
    }
    for n in &neutral {
        if lower.contains(n) {
            return ("neutral".to_string(), 0.6);
        }
    }
    ("unproductive".to_string(), 0.3)
}

// ── JWT refresh helper ───────────────────────────────────────────────────────

async fn refresh_jwt(
    client: &reqwest::Client,
    state: &Arc<Mutex<AgentState>>,
    cfg: &ResolvedConfig,
) -> Result<String> {
    info!("JWT expired or unauthorized, refreshing token...");
    let new_token = login_with_retry(client, &cfg.server, &cfg.auth_email, &cfg.auth_password).await?;
    let mut s = state.lock().await;
    s.jwt_token = new_token.clone();
    info!("JWT token refreshed successfully");
    Ok(new_token)
}

// ── Heartbeat loop ──────────────────────────────────────────────────────────

async fn heartbeat_loop(
    client: reqwest::Client,
    cfg: ResolvedConfig,
    state: Arc<Mutex<AgentState>>,
mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    let mut tick = interval(Duration::from_secs(60));
    loop {
        tokio::select! {
            _ = tick.tick() => {}
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    info!("Heartbeat loop shutting down");
                    return;
                }
            }
        }

        let (device_id, jwt_token) = {
            let s = state.lock().await;
            (s.device_id.clone(), s.jwt_token.clone())
        };

        let url = format!("{}/v1/devices/{}/heartbeat", cfg.server, device_id);
        match client
            .put(&url)
            .header("Authorization", format!("Bearer {}", jwt_token))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                info!(device_id, "Heartbeat OK");
                let mut s = state.lock().await;
                s.consecutive_heartbeat_failures = 0;
            }
            Ok(resp) if resp.status().as_u16() == 403 => {
                warn!("Heartbeat got 403 (device not approved), will re-enroll");
                let mut s = state.lock().await;
                s.consecutive_heartbeat_failures += 1;
            }
            Ok(resp) if resp.status().as_u16() == 401 => {
                warn!("Heartbeat got 401, attempting JWT refresh");
                match refresh_jwt(&client, &state, &cfg).await {
                    Ok(_) => {
                        info!("JWT refreshed, will retry heartbeat next cycle");
                    }
                    Err(e) => {
                        error!("Failed to refresh JWT: {}", e);
                    }
                }
                let mut s = state.lock().await;
                s.consecutive_heartbeat_failures += 1;
            }
            Ok(resp) => {
                let status = resp.status();
                warn!(status = status.as_u16(), "Heartbeat failed");
                let mut s = state.lock().await;
                s.consecutive_heartbeat_failures += 1;
            }
            Err(e) => {
                warn!("Heartbeat network error: {}", e);
                let mut s = state.lock().await;
                s.consecutive_heartbeat_failures += 1;
            }
        }

    
        let needs_reenroll = {
            let s = state.lock().await;
            s.consecutive_heartbeat_failures >= CONSECUTIVE_HB_FAILURES_FOR_REENROLL
        };

        if needs_reenroll {
            warn!(
                consecutive_failures = CONSECUTIVE_HB_FAILURES_FOR_REENROLL,
                "Too many consecutive heartbeat failures, re-enrolling device"
            );
            match reenroll(&client, &cfg, &state).await {
                Ok(_) => {
                    info!("Re-enrollment successful");
                    let mut s = state.lock().await;
                    s.consecutive_heartbeat_failures = 0;
                }
                Err(e) => {
                    error!("Re-enrollment failed: {}", e);
                }
            }
        }
    }
}

async fn reenroll(
    client: &reqwest::Client,
    cfg: &ResolvedConfig,
    state: &Arc<Mutex<AgentState>>,
) -> Result<()> {
    let jwt_token = {
        let s = state.lock().await;
        s.jwt_token.clone()
    };

    let enroll_resp =
        enroll_with_retry(client, &cfg.server, &cfg.employee_id, &cfg.company_id, &jwt_token).await?;

    let mut s = state.lock().await;
    s.device_id = enroll_resp.device_id.clone();
    s.device_token = enroll_resp.device_token.clone();
    info!(new_device_id = %enroll_resp.device_id, "Device re-enrolled with new ID");
    Ok(())
}

async fn wait_for_approval(
    client: &reqwest::Client,
    server: &str,
    device_id: &str,
) -> Result<()> {
    info!(device_id, "Waiting for admin approval...");
    let mut attempts = 0u32;
    loop {
        sleep(Duration::from_secs(10)).await;

        let url = format!("{}/v1/devices/{}/status", server, device_id);
        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                let body: serde_json::Value = resp.json().await.unwrap_or_default();
                let status = body.get("status").and_then(|s| s.as_str()).unwrap_or("unknown");
                match status {
                    "active" => {
                        info!("Device approved!");
                        return Ok(());
                    }
                    "rejected" => {
                        anyhow::bail!("Device enrollment was rejected by admin");
                    }
                    "pending" => {
                        attempts += 1;
                        if attempts % 6 == 0 {
                            info!(device_id, "Still waiting for approval... (status: pending)");
                        }
                    }
                    other => {
                        warn!(status = other, "Unknown device status, continuing to wait");
                        attempts += 1;
                    }
                }
            }
            Ok(resp) => {
                let status = resp.status();
                warn!(status = status.as_u16(), "Status check failed");
            }
            Err(e) => {
                warn!("Status check network error: {}", e);
            }
        }
    }
}

// ── Collect loop ────────────────────────────────────────────────────────────

async fn collect_loop(
    state: Arc<Mutex<AgentState>>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    let mut tick = interval(Duration::from_secs(10));
    let mut cpu_cache: Option<HashMap<i32, (u64, u64)>> = None;

    loop {
        tokio::select! {
            _ = tick.tick() => {}
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    if let Some(event) = state.lock().await.close_active_window() {
                        info!(app = %event.app_name, dur = event.duration_sec, "Closed active window on shutdown");
                        state.lock().await.push_events(vec![event]);
                    }
                    info!("Collect loop shutting down");
                    return;
                }
            }
        }

        let now = Utc::now();
        let idle_secs = agent_collectors::get_idle_seconds();
        let active_win = agent_collectors::get_active_window();
        let mut flush_events: Vec<AppUsageEventMeta> = Vec::new();

        {
            let mut s = state.lock().await;

            if idle_secs >= IDLE_THRESHOLD_SECS {
                if s.idle_since.is_none() {
                    s.idle_since = Some(now);
                    info!(idle_secs, "User went idle");
                }
                if let Some(event) = s.close_active_window() {
                    info!(app = %event.app_name, dur = event.duration_sec, "Closed active window due to idle");
                    flush_events.push(event);
                }
            } else {
                if s.idle_since.is_some() {
                    info!("User returned from idle");
                    s.idle_since = None;
                }
            }

            if idle_secs < IDLE_THRESHOLD_SECS {
                match active_win {
                    Some(win) => {
                        match &s.active_window {
                            Some(current) if current.app_name == win.process_name && current.window_title == win.title => {}
                            Some(_) => {
                                if let Some(event) = s.close_active_window() {
                                    info!(app = %event.app_name, dur = event.duration_sec, "Active window changed");
                                    flush_events.push(event);
                                }
                                s.active_window = Some(ActiveWindowSession {
                                    app_name: win.process_name.clone(),
                                    window_title: win.title.clone(),
                                    process_name: win.process_name.clone(),
                                    process_id: win.process_id,
                                    start_time: now,
                                });
                            }
                            None => {
                                info!(app = %win.process_name, title = %win.title, "New active window");
                                s.active_window = Some(ActiveWindowSession {
                                    app_name: win.process_name.clone(),
                                    window_title: win.title.clone(),
                                    process_name: win.process_name.clone(),
                                    process_id: win.process_id,
                                    start_time: now,
                                });
                            }
                        }
                    }
                    None => {
                        if let Some(event) = s.close_active_window() {
                            info!(app = %event.app_name, dur = event.duration_sec, "No active window, closing session");
                            flush_events.push(event);
                        }
                    }
                }
            }
        }

        let procs = agent_collectors::get_running_applications_with_cpu_cache(cpu_cache.as_ref());
        let new_cache = agent_collectors::build_cpu_cache(&procs);
        cpu_cache = Some(new_cache);

        let mut process_events = Vec::new();
        for proc_info in &procs {
            let (classification, confidence) = classify_process(&proc_info.name);
            let window_title = if proc_info.cmdline.is_empty() {
                proc_info.name.clone()
            } else {
                format!("{} - {}", proc_info.name, proc_info.cmdline)
            };
            process_events.push(AppUsageEventMeta {
                app_name: proc_info.name.clone(),
                window_title,
                process_name: proc_info.name.clone(),
                process_id: proc_info.pid,
                start_time: now,
                end_time: now,
                duration_sec: proc_info.cpu_percent,
                classification,
                confidence,
                role_id: None,
                device_id: String::new(),
            });
        }

        {
            let mut s = state.lock().await;
            for ev in &mut flush_events {
                ev.device_id = s.device_id.clone();
            }
            s.push_events(flush_events);

            for ev in &mut process_events {
                ev.device_id = s.device_id.clone();
            }
            s.push_events(process_events);

            if !procs.is_empty() {
                let app_summary: Vec<String> = procs.iter().take(5).map(|p| format!("{}({})", p.name, p.pid)).collect();
                info!(idle = idle_secs, apps = ?app_summary, "Collection tick");
            }
        }
    }
}

// ── Upload loop ────────────────────────────────────────────────────────────

async fn upload_loop(
    client: reqwest::Client,
    cfg: ResolvedConfig,
    state: Arc<Mutex<AgentState>>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    let mut tick = interval(Duration::from_secs(30));
    loop {
        tokio::select! {
            _ = tick.tick() => {}
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    info!("Upload loop shutting down");
                    return;
                }
            }
        }

        let events: Vec<AppUsageEventMeta> = {
            let mut s = state.lock().await;
            std::mem::take(&mut s.events)
        };

        if events.is_empty() {
            continue;
        }

        let (device_id, jwt_token) = {
            let s = state.lock().await;
            (s.device_id.clone(), s.jwt_token.clone())
        };

        if let Err(e) = upload_events(&client, &cfg, &state, &device_id, &jwt_token, events).await {
            error!("Upload cycle failed: {}", e);
        }
    }
}

async fn screenshot_loop(
    _state: Arc<Mutex<AgentState>>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    let commander = agent_screenshot::ScreenshotCommander::new();
    let mut tick = interval(Duration::from_secs(SCREENSHOT_INTERVAL_SECS));
    loop {
        tokio::select! {
            _ = tick.tick() => {}
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    info!("Screenshot loop shutting down");
                    return;
                }
            }
        }

        let idle_secs = agent_collectors::get_idle_seconds();
        if idle_secs >= IDLE_THRESHOLD_SECS {
            info!(idle_secs, "Skipping screenshot: user idle");
            continue;
        }

        match commander.capture().await {
            Ok(data) => {
                info!(size = data.len(), "Screenshot captured");
            }
            Err(e) => {
                info!("Screenshot skipped: {}", e);
            }
        }
    }
}

async fn upload_events(
    client: &reqwest::Client,
    cfg: &ResolvedConfig,
    state: &Arc<Mutex<AgentState>>,
    device_id: &str,
    jwt_token: &str,
    events: Vec<AppUsageEventMeta>,
) -> Result<()> {
    let mut summary_map: HashMap<String, (f64, u64, f64, f64, f64)> = HashMap::new();
    for ev in &events {
        let entry = summary_map
            .entry(ev.app_name.clone())
            .or_insert((0.0, 0, 0.0, 0.0, 0.0));
        entry.0 += ev.duration_sec;
        entry.1 += 1;
        match ev.classification.as_str() {
            "productive" => entry.2 += ev.duration_sec,
            "unproductive" => entry.3 += ev.duration_sec,
            "neutral" => entry.4 += ev.duration_sec,
            _ => {}
        }
    }

    for (app_name, (total_dur, count, prod, unprod, neutral)) in &summary_map {
        let summary = AppUsageSummary {
            device_id: device_id.to_string(),
            app_name: app_name.clone(),
            total_duration_sec: *total_dur,
            session_count: *count,
            productive_duration_sec: *prod,
            unproductive_duration_sec: *unprod,
            neutral_duration_sec: *neutral,
        };

        let meta: Vec<AppUsageEventMeta> = events
            .iter()
            .filter(|e| e.app_name == *app_name)
            .cloned()
            .collect();

        let meta_count = meta.len();
        let bulk = BulkEventRequest {
            device_id: device_id.to_string(),
            summary,
            metadata: meta,
        };

        match send_bulk_event(client, &cfg.server, jwt_token, &bulk).await {
            Ok(()) => {
                info!(app_name, meta_count, "Uploaded events for app");
            }
            Err(UploadError::Unauthorized) => {
                warn!("Upload got 401 for app '{}', refreshing JWT and retrying", app_name);
                match refresh_jwt(client, state, cfg).await {
                    Ok(new_token) => {
                        match send_bulk_event(client, &cfg.server, &new_token, &bulk).await {
                            Ok(()) => {
                                info!(app_name, meta_count, "Uploaded events for app after JWT refresh");
                            }
                            Err(_) => {
                                warn!("Upload retry failed for '{}', re-queuing events", app_name);
                                let mut s = state.lock().await;
                                s.requeue_events(
                                    events
                                        .iter()
                                        .filter(|e| e.app_name == *app_name)
                                        .cloned()
                                        .collect(),
                                );
                            }
                        }
                    }
                    Err(e) => {
                        error!("JWT refresh failed: {}, re-queuing events for '{}'", e, app_name);
                        let mut s = state.lock().await;
                        s.requeue_events(
                            events
                                .iter()
                                .filter(|e| e.app_name == *app_name)
                                .cloned()
                                .collect(),
                        );
                    }
                }
            }
            Err(UploadError::Failed(_msg)) => {
                warn!("Upload failed for '{}', re-queuing events", app_name);
                let mut s = state.lock().await;
                s.requeue_events(
                    events
                        .iter()
                        .filter(|e| e.app_name == *app_name)
                        .cloned()
                        .collect(),
                );
            }
        }
    }

    Ok(())
}

enum UploadError {
    Unauthorized,
    Failed(String),
}

async fn send_bulk_event(
    client: &reqwest::Client,
    server: &str,
    jwt_token: &str,
    bulk: &BulkEventRequest,
) -> std::result::Result<(), UploadError> {
    let resp = client
        .post(format!("{}/v1/events/bulk", server))
        .header("Authorization", format!("Bearer {}", jwt_token))
        .json(bulk)
        .send()
        .await;

    match resp {
        Ok(resp) => {
            if resp.status().is_success() {
                Ok(())
            } else if resp.status().as_u16() == 401 {
                Err(UploadError::Unauthorized)
            } else {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                warn!(status = status.as_u16(), %body, "Upload server error");
                Err(UploadError::Failed(format!("status {}", status)))
            }
        }
        Err(e) => {
            warn!("Upload network error: {}", e);
            sleep(Duration::from_secs(UPLOAD_RETRY_DELAY_SECS)).await;
            Err(UploadError::Failed(e.to_string()))
        }
    }
}

// ── Main ────────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    let args = Args::parse();

    // Handle service management commands first (no tokio runtime needed)
    if let Some(ref cmd) = args.service {
        match cmd {
            ServiceCommand::Install => return agent_service::install(),
            ServiceCommand::Uninstall => return agent_service::uninstall(),
            ServiceCommand::Start => return agent_service::start(),
            ServiceCommand::Stop => return agent_service::stop(),
        }
    }

    // Windows service mode: hand off to SCM
    if args.run_as_service {
        #[cfg(target_os = "windows")]
        {
            agent_service::set_agent_runner(Box::new(|| {
                let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
                rt.block_on(run_agent());
            }));
            return agent_service::run_service();
        }
        #[cfg(not(target_os = "windows"))]
        {
            return agent_service::run_service();
        }
    }

    // Normal CLI mode
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(run_agent())
}

async fn run_agent() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    info!("AINMS Agent v0.2.0 starting...");

    let args = Args::parse();
    let cfg = resolve_config(&args);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    info!(email = %cfg.auth_email, "Logging in...");
    let jwt_token = login_with_retry(&client, &cfg.server, &cfg.auth_email, &cfg.auth_password)
        .await
        .context("Failed to login after all retries")?;
    info!("Login successful");

    let enroll_resp = enroll_with_retry(&client, &cfg.server, &cfg.employee_id, &cfg.company_id, &jwt_token)
        .await
        .context("Failed to enroll after all retries")?;
    info!(device_id = %enroll_resp.device_id, status = %enroll_resp.status, "Enrolled");

    if enroll_resp.status == "pending" {
        wait_for_approval(&client, &cfg.server, &enroll_resp.device_id).await?;
    } else if enroll_resp.status == "rejected" {
        anyhow::bail!("Device enrollment was rejected by admin");
    }

    let state = Arc::new(Mutex::new(AgentState {
        device_id: enroll_resp.device_id.clone(),
        device_token: enroll_resp.device_token.clone(),
        jwt_token: jwt_token.clone(),
        events: Vec::new(),
        consecutive_heartbeat_failures: 0,
        active_window: None,
        idle_since: None,
    }));

    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    info!("Starting heartbeat, collector, and uploader loops...");

    let hb_client = client.clone();
    let hb_cfg = ResolvedConfig {
        employee_id: cfg.employee_id.clone(),
        company_id: cfg.company_id.clone(),
        server: cfg.server.clone(),
        auth_email: cfg.auth_email.clone(),
        auth_password: cfg.auth_password.clone(),
    };
    let hb_state = Arc::clone(&state);
    let hb_shutdown = shutdown_rx.clone();
    let heartbeat_handle = tokio::spawn(async move {
        heartbeat_loop(hb_client, hb_cfg, hb_state, hb_shutdown).await;
    });

    let collect_state = Arc::clone(&state);
    let collect_shutdown = shutdown_rx.clone();
    let collect_handle = tokio::spawn(async move {
        collect_loop(collect_state, collect_shutdown).await;
    });

    let upload_client = client.clone();
    let upload_cfg = ResolvedConfig {
        employee_id: cfg.employee_id.clone(),
        company_id: cfg.company_id.clone(),
        server: cfg.server.clone(),
        auth_email: cfg.auth_email.clone(),
        auth_password: cfg.auth_password.clone(),
    };
    let upload_state = Arc::clone(&state);
    let upload_shutdown = shutdown_rx.clone();
    let upload_handle = tokio::spawn(async move {
        upload_loop(upload_client, upload_cfg, upload_state, upload_shutdown).await;
    });

    let screenshot_state = Arc::clone(&state);
    let screenshot_shutdown = shutdown_rx.clone();
    let screenshot_handle = tokio::spawn(async move {
        screenshot_loop(screenshot_state, screenshot_shutdown).await;
    });

    info!("AINMS Agent running. Press Ctrl+C to stop.");

    tokio::signal::ctrl_c().await?;
    info!("\nShutting down gracefully...");

    // Signal all loops to stop
    let _ = shutdown_tx.send(true);

    {
        let events: Vec<AppUsageEventMeta> = {
            let mut s = state.lock().await;
            std::mem::take(&mut s.events)
        };
        if !events.is_empty() {
            info!(count = events.len(), "Attempting final upload of pending events...");
            let (device_id, jwt_token) = {
                let s = state.lock().await;
                (s.device_id.clone(), s.jwt_token.clone())
            };
            if let Err(e) =
                upload_events(&client, &cfg, &state, &device_id, &jwt_token, events).await
            {
                warn!("Final upload failed: {}", e);
            }
        }
    }

    tokio::time::sleep(Duration::from_millis(500)).await;
    heartbeat_handle.abort();
    collect_handle.abort();
    upload_handle.abort();
    screenshot_handle.abort();

    info!("AINMS Agent stopped.");
    Ok(())
}