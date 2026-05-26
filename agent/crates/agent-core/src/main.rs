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
use serde::Deserialize;
use tokio::sync::Mutex;
use tokio::time::{interval, sleep};
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

use agent_proto::events::{
    AppUsageEventMeta, AppUsageSummary, BulkEventRequest, EnrollmentResponse,
    PendingCommand, TokenEnrollRequest,
};
use agent_comms::socket::{self, SocketCommand};

use config::{default_config_path, load_state, save_state, AgentStateFile, AgentStateSection};

const AGENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const MAX_EVENT_BUFFER: usize = 10_000;
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
    Install {
        #[arg(long, help = "Install token for enrollment and authentication")]
        install_token: Option<String>,
        #[arg(long, help = "AINMS server URL")]
        server: Option<String>,
    },
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
    #[arg(long, help = "Install token for enrollment and authentication")]
    install_token: Option<String>,

    #[arg(long)]
    server: Option<String>,

    #[arg(long)]
    config: Option<String>,

    #[arg(long, hide = true)]
    run_as_service: bool,

    #[command(subcommand)]
    service: Option<ServiceCommand>,
}

#[derive(Debug, Deserialize)]
struct ConfigFile {
    server: Option<String>,
    install_token: Option<String>,
}

#[derive(Debug, Clone)]
struct ResolvedConfig {
    install_token: Option<String>,
    server: String,
    config_path: String,
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
                    server: None,
                    install_token: None,
                }
            })
        }
        None => ConfigFile {
            server: None,
            install_token: None,
        },
    };

    let install_token = args
        .install_token
        .clone()
        .or(file_cfg.install_token)
        .or_else(|| std::env::var("AINMS_INSTALL_TOKEN").ok());

    let server = args
        .server
        .clone()
        .or(file_cfg.server)
        .or_else(|| std::env::var("AINMS_SERVER").ok())
        .unwrap_or_else(|| "http://173.249.47.143:8440".to_string());

    let config_path = args
        .config
        .clone()
        .unwrap_or_else(default_config_path);

    ResolvedConfig {
        install_token,
        server,
        config_path,
    }
}

// ── API types ───────────────────────────────────────────────────────────────

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
    install_token: String,
    events: Vec<AppUsageEventMeta>,
    consecutive_heartbeat_failures: u32,
    active_window: Option<ActiveWindowSession>,
    idle_since: Option<chrono::DateTime<Utc>>,
    config_path: String,
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

    fn persist(&self, server: &str) {
        let state_file = AgentStateFile {
            agent: AgentStateSection {
                server: server.to_string(),
                install_token: self.install_token.clone(),
                device_id: self.device_id.clone(),
                device_token: self.device_token.clone(),
            },
        };
        if let Err(e) = save_state(&self.config_path, &state_file) {
            warn!("Failed to persist agent state: {}", e);
        }
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

async fn enroll_with_token(
    client: &reqwest::Client,
    server: &str,
    install_token: &str,
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

    let req = TokenEnrollRequest {
        install_token: install_token.to_string(),
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

    info!(install_token = &install_token[..8.min(install_token.len())], "Enrolling with install token");

    let resp = client
        .post(format!("{}/v1/enroll/token", server))
        .json(&req)
        .send()
        .await
        .context("Failed to send token enrollment request")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Token enrollment failed with status {}: {}", status, body);
    }

    let enroll_resp: EnrollmentResponse = resp.json().await.context("Failed to parse enrollment response")?;
    info!(device_id = %enroll_resp.device_id, status = %enroll_resp.status, "Enrolled with token");

    Ok(enroll_resp)
}

async fn enroll_with_token_retry(
    client: &reqwest::Client,
    server: &str,
    install_token: &str,
) -> Result<EnrollmentResponse> {
    retry_with_backoff(
        "token-enrollment",
        ENROLL_MAX_RETRIES,
        ENROLL_BASE_DELAY_SECS,
        || enroll_with_token(client, server, install_token),
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

        let (device_id, install_token) = {
            let s = state.lock().await;
            (s.device_id.clone(), s.install_token.clone())
        };

        let url = format!("{}/v1/devices/{}/heartbeat", cfg.server, device_id);
        let hb_body = serde_json::json!({ "agent_version": AGENT_VERSION });
        match client
            .put(&url)
            .header("Authorization", format!("Bearer {}", install_token))
            .json(&hb_body)
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
                warn!("Heartbeat got 401 (install token may be revoked)");
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
    let install_token = {
        let s = state.lock().await;
        s.install_token.clone()
    };

    if install_token.is_empty() {
        anyhow::bail!("No install token available for re-enrollment");
    }

    let enroll_resp = enroll_with_token_retry(client, &cfg.server, &install_token).await?;
    let mut s = state.lock().await;
    s.device_id = enroll_resp.device_id.clone();
    s.device_token = enroll_resp.device_token.clone();
    s.consecutive_heartbeat_failures = 0;
    s.persist(&cfg.server);
    info!(new_device_id = %enroll_resp.device_id, "Device re-enrolled with token");
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
                                    window_title: win.title,
                                    process_name: win.process_name.clone(),
                                    process_id: win.process_id,
                                    start_time: now,
                                });
                            }
                            None => {
                                info!(app = %win.process_name, title = %win.title, "New active window");
                                s.active_window = Some(ActiveWindowSession {
                                    app_name: win.process_name.clone(),
                                    window_title: win.title,
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

        let procs = agent_collectors::get_running_applications();
        cpu_cache = None;

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

        let (device_id, install_token) = {
            let s = state.lock().await;
            (s.device_id.clone(), s.install_token.clone())
        };

        if let Err(e) = upload_events(&client, &cfg.server, &state, &device_id, &install_token, events).await {
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
    server: &str,
    state: &Arc<Mutex<AgentState>>,
    device_id: &str,
    install_token: &str,
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

        match send_bulk_event(client, server, install_token, &bulk).await {
            Ok(()) => {
                info!(app_name, meta_count, "Uploaded events for app");
            }
            Err(_) => {
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
    Failed(String),
}

async fn send_bulk_event(
    client: &reqwest::Client,
    server: &str,
    install_token: &str,
    bulk: &BulkEventRequest,
) -> std::result::Result<(), UploadError> {
    let resp = client
        .post(format!("{}/v1/events/bulk", server))
        .header("Authorization", format!("Bearer {}", install_token))
        .json(bulk)
        .send()
        .await;

    match resp {
        Ok(resp) => {
            if resp.status().is_success() {
                Ok(())
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

#[allow(dead_code)]
async fn poll_commands(
    client: &reqwest::Client,
    server: &str,
    device_id: &str,
    install_token: &str,
) -> Result<Vec<PendingCommand>> {
    let url = format!("{}/v1/devices/{}/commands", server, device_id);
    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", install_token))
        .send()
        .await?;

    if resp.status().is_success() {
        let commands: Vec<PendingCommand> = resp.json().await?;
        Ok(commands)
    } else {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        warn!(status = status.as_u16(), "Command poll failed: {}", body);
        Ok(Vec::new())
    }
}

async fn handle_screenshot_request(
    client: &reqwest::Client,
    server: &str,
    device_id: &str,
    install_token: &str,
    cmd: &PendingCommand,
) -> Result<()> {
    let request_id = cmd.payload.get("request_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    info!(request_id, "Capturing screenshot on demand");

    let commander = agent_screenshot::ScreenshotCommander::new();
    let image_data = match commander.capture().await {
        Ok(data) => data,
        Err(e) => {
            error!("Screenshot capture failed: {}", e);
            return Err(e);
        }
    };

    info!(size = image_data.len(), "Screenshot captured, uploading");

    let url = format!("{}/v1/screenshot/upload", server);
    let file_part = reqwest::multipart::Part::bytes(image_data)
        .file_name("screenshot.png")
        .mime_str("image/png")
        .map_err(|e| anyhow::anyhow!("mime error: {}", e))?;

    let form = reqwest::multipart::Form::new()
        .text("request_id", request_id.to_string())
        .text("device_id", device_id.to_string())
        .part("image", file_part);

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", install_token))
        .multipart(form)
        .send()
        .await?;

    if resp.status().is_success() {
        info!(request_id, "Screenshot uploaded successfully");
    } else {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        warn!(status = status.as_u16(), "Screenshot upload failed: {}", body);
    }

    let ack_url = format!("{}/v1/commands/ack", server);
    let ack_resp = client
        .post(&ack_url)
        .header("Authorization", format!("Bearer {}", install_token))
        .json(&serde_json::json!({"command_id": cmd.id}))
        .send()
        .await?;

    if ack_resp.status().is_success() {
        info!(command_id = %cmd.id, "Command acknowledged");
    } else {
        warn!("Failed to acknowledge command");
    }

    Ok(())
}

async fn socket_command_loop(
    mut cmd_rx: tokio::sync::mpsc::Receiver<SocketCommand>,
    client: reqwest::Client,
    server: &str,
    device_id: &str,
    install_token: &str,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    loop {
        tokio::select! {
            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(SocketCommand::ScreenshotRequest { command_id, payload }) => {
                        info!(command_id = %command_id, "Processing screenshot_request from Socket.IO");
                        let pending_cmd = PendingCommand {
                            id: command_id,
                            device_id: device_id.to_string(),
                            command_type: "screenshot_request".to_string(),
                            payload,
                            status: "pending".to_string(),
                            created_at: chrono::Utc::now().to_rfc3339(),
                        };
                        let cmd_client = client.clone();
                        let cmd_server = server.to_string();
                        let cmd_device_id = device_id.to_string();
                        let cmd_install_token = install_token.to_string();
                        tokio::spawn(async move {
                            if let Err(e) = handle_screenshot_request(
                                &cmd_client, &cmd_server, &cmd_device_id, &cmd_install_token, &pending_cmd,
                            ).await {
                                error!("Failed to handle screenshot request: {}", e);
                            }
                        });
                    }
                    None => {
                        info!("Socket command channel closed");
                        return;
                    }
                }
            }
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    info!("Socket command loop shutting down");
                    return;
                }
            }
        }
    }
}

// ── Main ────────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    let args = Args::parse();

    if let Some(ref cmd) = args.service {
        match cmd {
            ServiceCommand::Install { install_token, server } => {
                let server = server.clone().unwrap_or_else(|| "http://173.249.47.143:8440".to_string());
                if let Some(token) = install_token {
                    let config_path = default_config_path();
                    if let Err(e) = config::write_initial_config(&config_path, &server, token) {
                        warn!("Failed to write initial config: {}", e);
                    }
                }
                return agent_service::install();
            }
            ServiceCommand::Uninstall => return agent_service::uninstall(),
            ServiceCommand::Start => return agent_service::start(),
            ServiceCommand::Stop => return agent_service::stop(),
        }
    }

    if args.run_as_service {
        #[cfg(target_os = "windows")]
        {
            agent_service::set_agent_runner(Box::new(|| {
                let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
                let _ = rt.block_on(run_agent());
            }));
            return agent_service::run_service();
        }
        #[cfg(not(target_os = "windows"))]
        {
            return agent_service::run_service();
        }
    }

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

    let (device_id, device_token, install_token) = match try_resume_or_enroll(&client, &cfg).await {
        Ok(result) => result,
        Err(e) => {
            anyhow::bail!("Failed to establish agent identity: {}. Run: ainms-agent --install-token <TOKEN> --server <URL>", e);
        }
    };

    let state = Arc::new(Mutex::new(AgentState {
        device_id: device_id.clone(),
        device_token: device_token.clone(),
        install_token: install_token.clone(),
        events: Vec::new(),
        consecutive_heartbeat_failures: 0,
        active_window: None,
        idle_since: None,
        config_path: cfg.config_path.clone(),
    }));

    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    let mut socket_cmd_handle: Option<tokio::task::JoinHandle<()>> = None;
    let mut socket_client_ref: Option<socket::SocketClient> = None;

    match socket::connect_socket(&cfg.server, &device_id, &install_token).await {
        Ok((sc, cmd_rx)) => {
            info!("Socket.IO connected");
            let cmd_client = client.clone();
            let cmd_server = cfg.server.clone();
            let cmd_install_token = install_token.clone();
            let cmd_device_id = device_id.clone();
            let cmd_shutdown = shutdown_rx.clone();
            socket_cmd_handle = Some(tokio::spawn(async move {
                socket_command_loop(cmd_rx, cmd_client, &cmd_server, &cmd_device_id, &cmd_install_token, cmd_shutdown).await;
            }));
            socket_client_ref = Some(sc);
        }
        Err(e) => {
            warn!("Socket.IO connection failed, commands will not be received in real-time: {}", e);
        }
    }

    info!("Starting heartbeat, collector, and uploader loops...");

    let hb_client = client.clone();
    let hb_cfg = cfg.clone();
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
    let upload_cfg = cfg.clone();
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

    let _ = shutdown_tx.send(true);

    {
        let events: Vec<AppUsageEventMeta> = {
            let mut s = state.lock().await;
            std::mem::take(&mut s.events)
        };
        if !events.is_empty() {
            info!(count = events.len(), "Attempting final upload of pending events...");
            let (device_id, install_token) = {
                let s = state.lock().await;
                (s.device_id.clone(), s.install_token.clone())
            };
            if let Err(e) =
                upload_events(&client, &cfg.server, &state, &device_id, &install_token, events).await
            {
                warn!("Final upload failed: {}", e);
            }
        }
    }

    if let Some(sc) = socket_client_ref {
        if let Err(e) = sc.disconnect().await {
            warn!("Socket.IO disconnect error: {}", e);
        }
    }

    tokio::time::sleep(Duration::from_millis(500)).await;
    heartbeat_handle.abort();
    collect_handle.abort();
    upload_handle.abort();
    screenshot_handle.abort();
    if let Some(h) = socket_cmd_handle {
        h.abort();
    }

    info!("AINMS Agent stopped.");
    Ok(())
}

async fn try_resume_or_enroll(
    client: &reqwest::Client,
    cfg: &ResolvedConfig,
) -> Result<(String, String, String)> {
    // Step 1: Try to resume from saved state
    if let Some(saved) = load_state(&cfg.config_path) {
        let a = &saved.agent;
        if !a.device_id.is_empty() && !a.device_token.is_empty() && !a.install_token.is_empty() {
            info!(device_id = %a.device_id, "Found saved state, trying heartbeat to resume...");
            match try_heartbeat(client, &cfg.server, &a.device_id, &a.install_token).await {
                Ok(()) => {
                    info!("Resumed with saved device identity (heartbeat OK)");
                    return Ok((a.device_id.clone(), a.device_token.clone(), a.install_token.clone()));
                }
                Err(e) => {
                    warn!("Saved state heartbeat failed: {}, proceeding to re-enroll", e);
                }
            }
        }
    }

    // Step 2: Enroll with install token
    if let Some(ref install_token) = cfg.install_token {
        if !install_token.is_empty() {
            info!("Enrolling with install token...");
            let enroll_resp = enroll_with_token_retry(client, &cfg.server, install_token).await?;

            if enroll_resp.status == "pending" {
                wait_for_approval(client, &cfg.server, &enroll_resp.device_id).await?;
            } else if enroll_resp.status == "rejected" {
                anyhow::bail!("Device enrollment was rejected by admin");
            }

            let state_file = AgentStateFile {
                agent: AgentStateSection {
                    server: cfg.server.clone(),
                    install_token: install_token.clone(),
                    device_id: enroll_resp.device_id.clone(),
                    device_token: enroll_resp.device_token.clone(),
                },
            };
            if let Err(e) = save_state(&cfg.config_path, &state_file) {
                warn!("Failed to save state after enrollment: {}", e);
            }

            return Ok((enroll_resp.device_id, enroll_resp.device_token, install_token.clone()));
        }
    }

    // Step 3: No token, no saved state
    anyhow::bail!("No install token provided and no saved state found. Run: ainms-agent --install-token <TOKEN> --server <URL>");
}

async fn try_heartbeat(
    client: &reqwest::Client,
    server: &str,
    device_id: &str,
    install_token: &str,
) -> Result<()> {
    let url = format!("{}/v1/devices/{}/heartbeat", server, device_id);
    let resp = client
        .put(&url)
        .header("Authorization", format!("Bearer {}", install_token))
        .send()
        .await
        .context("Heartbeat request failed")?;

    if resp.status().is_success() {
        Ok(())
    } else {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Heartbeat failed with status {}: {}", status, body);
    }
}