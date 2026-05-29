mod config;
mod dialog;
pub(crate) mod os;
mod rule_engine;

use std::collections::HashMap;
use std::collections::HashSet;
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
    AppUsageEventMeta, AppUsageSummary, BulkEventRequest, BulkNetworkEventRequest,
    EnrollmentResponse, NetworkConnection, NetworkTrafficSummary, PendingCommand,
    TokenEnrollRequest,
};
use agent_comms::socket::{self, SocketCommand};
use agent_store::Store;

use config::{default_config_path, load_state, save_state, AgentStateFile, AgentStateSection};
use rule_engine::{EnforcementAction, RuleEngine};

const AGENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const MAX_EVENT_BUFFER: usize = 10_000;
const ENROLL_MAX_RETRIES: u32 = 5;
const ENROLL_BASE_DELAY_SECS: u64 = 2;
const CONSECUTIVE_HB_FAILURES_FOR_REENROLL: u32 = 3;
const UPLOAD_RETRY_DELAY_SECS: u64 = 5;
const IDLE_THRESHOLD_SECS: f64 = 300.0;
const SCREENSHOT_INTERVAL_SECS: u64 = 300;
const NETWORK_INTERVAL_SECS: u64 = 60;

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

    #[arg(long, hide = true)]
    take_screenshot: bool,

    #[arg(long, hide = true)]
    dialog_notify: bool,

    #[arg(long, hide = true)]
    dialog_ask: bool,

    #[arg(long, hide = true)]
    dialog_prompt: bool,

    #[arg(long, hide = true)]
    dialog_title: Option<String>,

    #[arg(long, hide = true)]
    dialog_message: Option<String>,

    #[arg(long)]
    device_id: Option<String>,

    #[arg(long)]
    request_id: Option<String>,

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
        .unwrap_or_else(|| "http://localhost:8440".to_string());

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
    network_connections: Vec<NetworkConnection>,
    consecutive_heartbeat_failures: u32,
    active_window: Option<ActiveWindowSession>,
    idle_since: Option<chrono::DateTime<Utc>>,
    config_path: String,
    screenshot_enabled: bool,
    screenshot_interval_secs: u64,
    rule_engine: RuleEngine,
    store: Store,
}

impl AgentState {
    fn push_events(&mut self, new_events: Vec<AppUsageEventMeta>) {
        // Persist to SQLite immediately for durability
        if let Err(e) = self.store.insert_events(&new_events) {
            warn!("Failed to persist events to store: {}", e);
        }

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
        let result = agent_ml::classify_keyword_fallback(&session.process_name);
        let classification = result.category;
        let confidence = result.confidence;
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

        let mut process_events = Vec::new();
        for proc_info in &procs {
            let result = agent_ml::classify_keyword_fallback(&proc_info.name);
            let classification = result.category;
            let confidence = result.confidence;
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

async fn network_upload_loop(
    client: reqwest::Client,
    cfg: ResolvedConfig,
    state: Arc<Mutex<AgentState>>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    let mut tick = interval(Duration::from_secs(120));
    loop {
        tokio::select! {
            _ = tick.tick() => {}
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    info!("Network upload loop shutting down");
                    return;
                }
            }
        }

        let connections: Vec<NetworkConnection> = {
            let mut s = state.lock().await;
            std::mem::take(&mut s.network_connections)
        };

        if connections.is_empty() {
            continue;
        }

        let (device_id, install_token) = {
            let s = state.lock().await;
            (s.device_id.clone(), s.install_token.clone())
        };

        if let Err(e) = upload_network_connections(&client, &cfg.server, &state, &device_id, &install_token, connections).await {
            error!("Network upload cycle failed: {}", e);
        }
    }
}

async fn browser_tabs_loop(
    client: reqwest::Client,
    cfg: ResolvedConfig,
    state: Arc<Mutex<AgentState>>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    let monitor = agent_collectors::BrowserTabMonitor::new();
    let mut tick = interval(Duration::from_secs(30));
    loop {
        tokio::select! {
            _ = tick.tick() => {}
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    info!("Browser tabs loop shutting down");
                    return;
                }
            }
        }

        let tabs = monitor.get_all_tabs().await;
        if tabs.is_empty() {
            continue;
        }

        let (device_id, install_token) = {
            let s = state.lock().await;
            (s.device_id.clone(), s.install_token.clone())
        };

        let tab_infos: Vec<agent_proto::events::BrowserTabInfo> = tabs.into_iter().map(|t| {
            agent_proto::events::BrowserTabInfo {
                title: t.title,
                url: t.url,
                browser: t.browser,
                active: t.active,
            }
        }).collect();

        let bulk = agent_proto::events::BulkBrowserTabRequest {
            device_id: device_id.clone(),
            tabs: tab_infos,
        };

        let resp = client
            .post(format!("{}/v1/events/browser-tabs", cfg.server))
            .header("Authorization", format!("Bearer {}", install_token))
            .json(&bulk)
            .send()
            .await;

        match resp {
            Ok(resp) if resp.status().is_success() => {
                info!("Browser tabs uploaded");
            }
            Ok(resp) => {
                warn!(status = resp.status().as_u16(), "Browser tabs upload failed");
            }
            Err(e) => {
                warn!("Browser tabs upload error: {}", e);
            }
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

        #[cfg(target_os = "windows")]
        if agent_screenshot::is_session_zero() {
            info!("Skipping periodic screenshot: running in Session 0 (service context)");
            continue;
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

async fn screenshot_interval_loop(
    client: reqwest::Client,
    cfg: ResolvedConfig,
    state: Arc<Mutex<AgentState>>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    let mut tick = interval(Duration::from_secs(SCREENSHOT_INTERVAL_SECS));
    tick.tick().await; // skip first immediate tick

    loop {
        tokio::select! {
            _ = tick.tick() => {
                let (screenshot_enabled, interval_secs, device_id, install_token, server) = {
                    let s = state.lock().await;
                    (
                        s.screenshot_enabled,
                        s.screenshot_interval_secs,
                        s.device_id.clone(),
                        s.install_token.clone(),
                        cfg.server.clone(),
                    )
                };

                if !screenshot_enabled {
                    continue;
                }

                // Reset the tick interval if it differs from the current interval
                if interval_secs != SCREENSHOT_INTERVAL_SECS && interval_secs > 0 {
                    tick = interval(Duration::from_secs(interval_secs));
                    tick.tick().await; // skip immediate
                }

                let idle_secs = agent_collectors::get_idle_seconds();
                if idle_secs >= IDLE_THRESHOLD_SECS {
                    info!(idle_secs, "Skipping auto screenshot: user idle");
                    continue;
                }

                info!("Auto-capturing screenshot (interval: {}s)", interval_secs);

                // In Session 0 (service context), we cannot capture the screen directly.
                // Spawn a helper process in the user's interactive session; it will
                // capture the screenshot and upload it to the server itself.
                #[cfg(target_os = "windows")]
                if agent_screenshot::is_session_zero() {
                    let request_id = format!("auto-{}", chrono::Utc::now().timestamp());
                    match agent_screenshot::ScreenshotCommander::capture_in_user_session(
                        &server, &device_id, &install_token, &request_id,
                    ) {
                        Ok(()) => info!("Auto screenshot helper spawned in user session"),
                        Err(e) => warn!("Failed to spawn auto screenshot helper: {}", e),
                    }
                    continue;
                }

                let commander = agent_screenshot::ScreenshotCommander::new();
                match commander.capture().await {
                    Ok(image_data) => {
                        info!(size = image_data.len(), "Auto screenshot captured, uploading");
                        let url = format!("{}/v1/screenshot/upload", server);
                        let file_part = reqwest::multipart::Part::bytes(image_data)
                            .file_name("screenshot.png")
                            .mime_str("image/png");
                        match file_part {
                            Ok(part) => {
                                let form = reqwest::multipart::Form::new()
                                    .text("request_id", format!("auto-{}", chrono::Utc::now().timestamp()))
                                    .text("device_id", device_id.clone())
                                    .part("image", part);

                                match client
                                    .post(&url)
                                    .header("Authorization", format!("Bearer {}", install_token))
                                    .multipart(form)
                                    .send()
                                    .await
                                {
                                    Ok(resp) if resp.status().is_success() => {
                                        info!("Auto screenshot uploaded successfully");
                                    }
                                    Ok(resp) => {
                                        let status = resp.status();
                                        warn!(status = status.as_u16(), "Auto screenshot upload failed");
                                    }
                                    Err(e) => {
                                        warn!("Auto screenshot upload error: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Auto screenshot mime error: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Auto screenshot capture failed: {}", e);
                    }
                }
            }
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    info!("Screenshot interval loop shutting down");
                    return;
                }
            }
        }
    }
}

async fn network_loop(
    state: Arc<Mutex<AgentState>>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    let collector = agent_collectors::NetworkCollector::new();
    let mut tick = interval(Duration::from_secs(NETWORK_INTERVAL_SECS));

    loop {
        tokio::select! {
            _ = tick.tick() => {}
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    info!("Network loop shutting down");
                    return;
                }
            }
        }

        let mut connections = agent_collectors::get_network_connections();

        collector.resolve_ips(&mut connections).await;

        let resolved_count = connections.iter().filter(|c| c.remote_hostname.is_some()).count();
        let total_count = connections.len();

        if !connections.is_empty() {
            info!(total = total_count, resolved = resolved_count, "Network connections collected");
        }

        // Classify network URLs via rule engine
        {
            let mut s = state.lock().await;
            for conn in &connections {
                if let Some(ref url) = conn.reconstructed_url {
                    if !url.is_empty() {
                        let url_result = s.rule_engine.evaluate_url(url, &conn.process_name);
                        if url_result.category == "unproductive" || url_result.enforcement != EnforcementAction::None {
                            warn!(
                                url = %url,
                                category = %url_result.category,
                                confidence = %url_result.confidence,
                                "Unproductive URL detected"
                            );
                        }
                    }
                }
            }
        }

        {
            let mut s = state.lock().await;
            let overflow = s.network_connections.len() + connections.len();
            if overflow > MAX_EVENT_BUFFER {
                let to_drop = overflow.saturating_sub(MAX_EVENT_BUFFER);
                let drop_count = to_drop.min(s.network_connections.len());
                warn!(to_drop, "Network buffer full, dropping {} oldest connections", drop_count);
                s.network_connections.drain(..drop_count);
            }
            s.network_connections.extend(connections);
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
    Failed,
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
                Err(UploadError::Failed)
            }
        }
        Err(e) => {
            warn!("Upload network error: {}", e);
            sleep(Duration::from_secs(UPLOAD_RETRY_DELAY_SECS)).await;
            Err(UploadError::Failed)
        }
    }
}

async fn upload_network_connections(
    client: &reqwest::Client,
    server: &str,
    _state: &Arc<Mutex<AgentState>>,
    device_id: &str,
    install_token: &str,
    connections: Vec<NetworkConnection>,
) -> Result<()> {
    let mut total = 0u32;
    let mut resolved = 0u32;
    let mut tcp_count = 0u32;
    let mut udp_count = 0u32;
    let mut domains = HashSet::new();

    for conn in &connections {
        total += 1;
        if conn.remote_hostname.is_some() {
            resolved += 1;
            if let Some(ref host) = conn.remote_hostname {
                domains.insert(host.clone());
            }
        }
        match conn.protocol.as_str() {
            "tcp" => tcp_count += 1,
            "udp" => udp_count += 1,
            _ => {}
        }
    }

    let summary = NetworkTrafficSummary {
        device_id: device_id.to_string(),
        total_connections: total,
        resolved_connections: resolved,
        unique_domains: domains.into_iter().collect(),
        protocol_breakdown: serde_json::json!({
            "tcp": tcp_count,
            "udp": udp_count,
        }),
    };

    let bulk = BulkNetworkEventRequest {
        device_id: device_id.to_string(),
        summary,
        connections,
    };

    let resp = client
        .post(format!("{}/v1/events/network", server))
        .header("Authorization", format!("Bearer {}", install_token))
        .json(&bulk)
        .send()
        .await;

    match resp {
        Ok(resp) => {
            if resp.status().is_success() {
                info!("Network events uploaded successfully");
                Ok(())
            } else {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                warn!(status = status.as_u16(), %body, "Network upload server error");
                Err(anyhow::anyhow!("Network upload failed with status {}", status))
            }
        }
        Err(e) => {
            warn!("Network upload network error: {}", e);
            Err(anyhow::anyhow!("Network upload request failed: {}", e))
        }
    }
}

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

    info!(request_id, "Processing screenshot_request");

    #[cfg(target_os = "windows")]
    if agent_screenshot::is_session_zero() {
        info!(request_id, "Running in Session 0, spawning helper in user session");
        match agent_screenshot::ScreenshotCommander::capture_in_user_session(
            server, device_id, install_token, request_id,
        ) {
            Ok(()) => info!(request_id, "Screenshot helper spawned in user session"),
            Err(e) => error!("Failed to spawn screenshot helper: {}", e),
        }
        let ack_url = format!("{}/v1/commands/ack", server);
        let _ = client.post(&ack_url)
            .header("Authorization", format!("Bearer {}", install_token))
            .json(&serde_json::json!({"command_id": cmd.id}))
            .send()
            .await;
        return Ok(());
    }

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
    state: Arc<Mutex<AgentState>>,
    socket_client: socket::SocketClient,
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
                    Some(SocketCommand::PolicyUpdate { command_id, payload }) => {
                        info!(command_id = %command_id, "Processing PolicyUpdate command");
                        if let Ok(rules) = serde_json::from_value::<agent_proto::events::RulesInfo>(payload.clone()) {
                            let mut s = state.lock().await;
                            s.rule_engine.update_rules(rules);
                            info!(command_id = %command_id, "Rule engine updated from PolicyUpdate");
                        } else {
                            warn!(command_id = %command_id, "Failed to parse PolicyUpdate payload as RulesInfo");
                        }
                    }
                    Some(SocketCommand::NLQuery { query_id, query, payload: _ }) => {
                        info!(query_id = %query_id, query = %query, "Processing NLQuery command");

                        // Gather data from state and release the lock before emitting
                        let (report, emit_socket) = {
                            let s = state.lock().await;

                            // Collect last 50 events
                            let recent_events: Vec<&AppUsageEventMeta> =
                                s.events.iter().rev().take(50).collect();
                            let total_events = recent_events.len();

                            // Classify events by classification
                            let mut class_counts: HashMap<String, usize> = HashMap::new();
                            let mut class_durations: HashMap<String, f64> = HashMap::new();
                            for evt in &recent_events {
                                let cls = evt.classification.to_lowercase();
                                *class_counts.entry(cls.clone()).or_insert(0) += 1;
                                *class_durations.entry(cls.clone()).or_insert(0.0) += evt.duration_sec;
                            }

                            // Top apps by total duration
                            let mut app_durations: HashMap<String, (f64, String)> = HashMap::new();
                            for evt in &recent_events {
                                let entry = app_durations.entry(evt.app_name.clone()).or_insert((0.0, evt.classification.clone()));
                                entry.0 += evt.duration_sec;
                            }
                            let mut apps_vec: Vec<(String, f64, String)> = app_durations
                                .into_iter()
                                .map(|(name, (dur, cls))| (name, dur, cls))
                                .collect();
                            apps_vec.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                            let top_apps: Vec<serde_json::Value> = apps_vec
                                .iter()
                                .take(10)
                                .map(|(name, dur, cls)| {
                                    serde_json::json!({
                                        "app_name": name,
                                        "duration_sec": *dur,
                                        "classification": cls,
                                    })
                                })
                                .collect();

                            // Network connections (last 20)
                            let recent_net: Vec<&NetworkConnection> =
                                s.network_connections.iter().rev().take(20).collect();
                            let unique_domains: Vec<String> = {
                                let mut domains: HashSet<String> = HashSet::new();
                                for conn in &recent_net {
                                    if let Some(ref hostname) = conn.remote_hostname {
                                        if !hostname.is_empty() {
                                            domains.insert(hostname.clone());
                                        }
                                    }
                                }
                                let mut d: Vec<String> = domains.into_iter().collect();
                                d.sort();
                                d
                            };

                            // Role info from rule engine
                            let role_info: Option<serde_json::Value> = s.rule_engine.get_rules().and_then(|r| {
                                r.role.as_ref().map(|role| {
                                    serde_json::json!({
                                        "name": role.name,
                                        "work_description": role.work_description,
                                    })
                                })
                            });

                            // Build classification breakdown
                            let classification_breakdown = serde_json::json!({
                                "productive": {
                                    "count": class_counts.get("productive").copied().unwrap_or(0),
                                    "duration_sec": class_durations.get("productive").copied().unwrap_or(0.0),
                                },
                                "unproductive": {
                                    "count": class_counts.get("unproductive").copied().unwrap_or(0),
                                    "duration_sec": class_durations.get("unproductive").copied().unwrap_or(0.0),
                                },
                                "neutral": {
                                    "count": class_counts.get("neutral").copied().unwrap_or(0),
                                    "duration_sec": class_durations.get("neutral").copied().unwrap_or(0.0),
                                },
                            });

                            // Determine dominant classification
                            let prod_dur = class_durations.get("productive").copied().unwrap_or(0.0);
                            let unprod_dur = class_durations.get("unproductive").copied().unwrap_or(0.0);
                            let neutral_dur = class_durations.get("neutral").copied().unwrap_or(0.0);
                            let dominant = if prod_dur >= unprod_dur && prod_dur >= neutral_dur {
                                "productive".to_string()
                            } else if unprod_dur >= prod_dur && unprod_dur >= neutral_dur {
                                "unproductive".to_string()
                            } else {
                                "neutral".to_string()
                            };

                            let total_duration: f64 = class_durations.values().sum();
                            let dominant_pct = if total_duration > 0.0 {
                                ((class_durations.get(&dominant).copied().unwrap_or(0.0) / total_duration) * 100.0).round() as u32
                            } else {
                                0
                            };

                            // Build report_text programmatically
                            let mut text_parts: Vec<String> = Vec::new();

                            text_parts.push(format!(
                                "Based on recent activity, this employee has been primarily {} ({}% of tracked time).",
                                dominant, dominant_pct
                            ));

                            if !top_apps.is_empty() {
                                let app_list: Vec<String> = apps_vec
                                    .iter()
                                    .take(10)
                                    .map(|(name, dur, _)| format!("{} ({}min)", name, (*dur / 60.0).round() as u32))
                                    .collect();
                                text_parts.push(format!("Top applications: {}.", app_list.join(", ")));
                            }

                            if !unique_domains.is_empty() {
                                text_parts.push(format!(
                                    "Network activity shows connections to {}.",
                                    unique_domains.join(", ")
                                ));
                            }

                            if let Some(ref role) = role_info {
                                let role_name = role.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown");
                                let work_desc = role.get("work_description").and_then(|v| v.as_str()).unwrap_or("");
                                text_parts.push(format!("Role: {} — {}.", role_name, work_desc));
                            }

                            let unprod_count = class_counts.get("unproductive").copied().unwrap_or(0);
                            let unprod_dur = class_durations.get("unproductive").copied().unwrap_or(0.0);
                            if unprod_count > 0 {
                                text_parts.push(format!(
                                    "Warning: {} unproductive events detected totaling {} minutes.",
                                    unprod_count,
                                    (unprod_dur / 60.0).round() as u32
                                ));
                            }

                            let report_text = text_parts.join(" ");

                            let mut summary = serde_json::json!({
                                "total_events": total_events,
                                "classification_breakdown": classification_breakdown,
                                "top_apps": top_apps,
                                "network_summary": {
                                    "total_connections": recent_net.len(),
                                    "unique_domains": unique_domains,
                                },
                            });

                            if let Some(role) = role_info {
                                summary.as_object_mut().unwrap().insert("role".to_string(), role);
                            }

                            let report = serde_json::json!({
                                "query_id": query_id,
                                "device_id": s.device_id,
                                "query": query,
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                                "summary": summary,
                                "report_text": report_text,
                            });

                            (report, socket_client.clone())
                        }; // state lock released here

                        match emit_socket.emit("agent_report", report).await {
                            Ok(()) => {
                                info!(query_id = %query_id, "agent_report emitted successfully");
                            }
                            Err(e) => {
                                error!(query_id = %query_id, "Failed to emit agent_report: {}", e);
                            }
                        }
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

    if args.take_screenshot {
        return run_take_screenshot(&args);
    }

    // Dialog helper mode: spawned by CreateProcessAsUserW from Session 0
    if args.dialog_notify || args.dialog_ask || args.dialog_prompt {
        return run_dialog_helper_cmd(&args);
    }

    if let Some(ref cmd) = args.service {
        match cmd {
            ServiceCommand::Install { install_token, server } => {
                let server = server.clone().unwrap_or_else(|| "http://localhost:8440".to_string());
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

fn run_take_screenshot(args: &Args) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::System::Console::FreeConsole;
        unsafe {
            let _ = FreeConsole();
        }
    }

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .init();

        let server = args.server.as_deref().unwrap_or("http://localhost:8440");
        let device_id = args.device_id.as_deref().ok_or_else(|| anyhow::anyhow!("--device-id required"))?;
        let install_token = args.install_token.as_deref().ok_or_else(|| anyhow::anyhow!("--install-token required"))?;
        let request_id = args.request_id.as_deref().ok_or_else(|| anyhow::anyhow!("--request-id required"))?;

        info!(request_id, "Screenshot helper started in user session");

        let commander = agent_screenshot::ScreenshotCommander::new();
        let image_data = commander.capture().await
            .map_err(|e| anyhow::anyhow!("Capture failed: {}", e))?;

        info!(size = image_data.len(), "Screenshot captured, uploading");

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        let url = format!("{}/v1/screenshot/upload", server);
        let file_part = reqwest::multipart::Part::bytes(image_data)
            .file_name("screenshot.png")
            .mime_str("image/png")
            .map_err(|e| anyhow::anyhow!("mime error: {}", e))?;
        let form = reqwest::multipart::Form::new()
            .text("request_id", request_id.to_string())
            .text("device_id", device_id.to_string())
            .part("image", file_part);

        let resp = client.post(&url)
            .header("Authorization", format!("Bearer {}", install_token))
            .multipart(form)
            .send()
            .await?;

        if resp.status().is_success() {
            info!(request_id, "Screenshot uploaded successfully");
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            error!(status = status.as_u16(), "Screenshot upload failed: {}", body);
        }

        Ok(())
    })
}

fn run_dialog_helper_cmd(args: &Args) -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let dialog_type = if args.dialog_notify {
        "notify"
    } else if args.dialog_ask {
        "ask"
    } else if args.dialog_prompt {
        "prompt"
    } else {
        anyhow::bail!("No dialog type flag specified");
    };

    let title = args.dialog_title.as_deref().unwrap_or("AINMS Agent");
    let message = args.dialog_message.as_deref().unwrap_or("");

    info!(dialog_type, title, "Dialog helper started in user session");

    dialog::run_dialog_helper(dialog_type, title, message)?;

    Ok(())
}

async fn run_agent() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    info!("AINMS Agent v0.2.0 starting...");

    let args = Args::parse();
    let cfg = resolve_config(&args);

    let db_path = agent_store::default_db_path();
    let store = Store::open(std::path::Path::new(&db_path))
        .unwrap_or_else(|e| {
            warn!("Failed to open database: {}, continuing with fallback", e);
            Store::new()
        });

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    let (device_id, device_token, install_token, rules) = match try_resume_or_enroll(&client, &cfg, &store).await {
        Ok(result) => result,
        Err(e) => {
            anyhow::bail!("Failed to establish agent identity: {}. Run: ainms-agent --install-token <TOKEN> --server <URL>", e);
        }
    };

    let (screenshot_enabled, screenshot_interval_secs) = if let Some(ref r) = rules {
        (
            r.policy.screenshot_enabled,
            if r.policy.upload_interval > 0 { r.policy.upload_interval as u64 } else { SCREENSHOT_INTERVAL_SECS },
        )
    } else {
        (false, SCREENSHOT_INTERVAL_SECS)
    };

    if screenshot_enabled {
        info!(interval_secs = screenshot_interval_secs, "Auto screenshot enabled from enrollment policy");
    }

    let state = Arc::new(Mutex::new(AgentState {
        device_id: device_id.clone(),
        device_token: device_token.clone(),
        install_token: install_token.clone(),
        events: Vec::new(),
        network_connections: Vec::new(),
        consecutive_heartbeat_failures: 0,
        active_window: None,
        idle_since: None,
        config_path: cfg.config_path.clone(),
        screenshot_enabled,
        screenshot_interval_secs,
        rule_engine: {
            let mut re = RuleEngine::new();
            if let Some(r) = rules {
                re.update_rules(r);
            }
            re
        },
        store,
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
            let cmd_state = Arc::clone(&state);
            let cmd_socket_client = sc.clone();
            socket_cmd_handle = Some(tokio::spawn(async move {
                socket_command_loop(cmd_rx, cmd_client, &cmd_server, &cmd_device_id, &cmd_install_token, cmd_state, cmd_socket_client, cmd_shutdown).await;
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

    let si_client = client.clone();
    let si_cfg = cfg.clone();
    let si_state = Arc::clone(&state);
    let si_shutdown = shutdown_rx.clone();
    let screenshot_interval_handle = tokio::spawn(async move {
        screenshot_interval_loop(si_client, si_cfg, si_state, si_shutdown).await;
    });

    let network_state = Arc::clone(&state);
    let network_shutdown = shutdown_rx.clone();
    let network_handle = tokio::spawn(async move {
        network_loop(network_state, network_shutdown).await;
    });

    let net_upload_client = client.clone();
    let net_upload_cfg = cfg.clone();
    let net_upload_state = Arc::clone(&state);
    let net_upload_shutdown = shutdown_rx.clone();
    let net_upload_handle = tokio::spawn(async move {
        network_upload_loop(net_upload_client, net_upload_cfg, net_upload_state, net_upload_shutdown).await;
    });

    let browser_client = client.clone();
    let browser_cfg = cfg.clone();
    let browser_state = Arc::clone(&state);
    let browser_shutdown = shutdown_rx.clone();
    let browser_handle = tokio::spawn(async move {
        browser_tabs_loop(browser_client, browser_cfg, browser_state, browser_shutdown).await;
    });

    let poll_client = client.clone();
    let poll_cfg = cfg.clone();
    let poll_state = Arc::clone(&state);
    let mut poll_shutdown = shutdown_rx.clone();
    let poll_handle = tokio::spawn(async move {
        let mut tick = interval(Duration::from_secs(30));
        tick.tick().await; // skip first
        loop {
            tokio::select! {
                _ = tick.tick() => {
                    let (device_id, install_token) = {
                        let s = poll_state.lock().await;
                        (s.device_id.clone(), s.install_token.clone())
                    };
                    match poll_commands(&poll_client, &poll_cfg.server, &device_id, &install_token).await {
                        Ok(commands) => {
                            for cmd in &commands {
                                info!(cmd_id = %cmd.id, cmd_type = %cmd.command_type, "Polled pending command");
                            }
                        }
                        Err(e) => warn!("Command poll error: {}", e),
                    }
                }
                _ = poll_shutdown.changed() => {
                    if *poll_shutdown.borrow() {
                        info!("Command poll loop shutting down");
                        return;
                    }
                }
            }
        }
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

    {
        let connections: Vec<NetworkConnection> = {
            let mut s = state.lock().await;
            std::mem::take(&mut s.network_connections)
        };
        if !connections.is_empty() {
            info!(count = connections.len(), "Attempting final upload of pending network connections...");
            let (device_id, install_token) = {
                let s = state.lock().await;
                (s.device_id.clone(), s.install_token.clone())
            };
            if let Err(e) =
                upload_network_connections(&client, &cfg.server, &state, &device_id, &install_token, connections).await
            {
                warn!("Final network upload failed: {}", e);
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
    screenshot_interval_handle.abort();
    network_handle.abort();
    net_upload_handle.abort();
    browser_handle.abort();
    poll_handle.abort();
    if let Some(h) = socket_cmd_handle {
        h.abort();
    }

    info!("AINMS Agent stopped.");
    Ok(())
}

async fn try_resume_or_enroll(
    client: &reqwest::Client,
    cfg: &ResolvedConfig,
    store: &Store,
) -> Result<(String, String, String, Option<agent_proto::events::RulesInfo>)> {
    // Step 1: Try to resume from saved state
    if let Some(saved) = load_state(&cfg.config_path) {
        let a = &saved.agent;
        if !a.device_id.is_empty() && !a.device_token.is_empty() && !a.install_token.is_empty() {
            info!(device_id = %a.device_id, "Found saved state, trying heartbeat to resume...");
            match try_heartbeat(client, &cfg.server, &a.device_id, &a.install_token).await {
                Ok(()) => {
                    info!("Resumed with saved device identity (heartbeat OK)");
                    if let Err(e) = store.save_agent_state(&cfg.server, &a.install_token, &a.device_id, &a.device_token) {
                        warn!("Failed to persist resumed state to database: {}", e);
                    }
                    return Ok((a.device_id.clone(), a.device_token.clone(), a.install_token.clone(), None));
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

            if let Err(e) = store.save_agent_state(&cfg.server, install_token, &enroll_resp.device_id, &enroll_resp.device_token) {
                warn!("Failed to save agent state to database: {}", e);
            }

            let rules = enroll_resp.rules.clone();
            return Ok((enroll_resp.device_id, enroll_resp.device_token, install_token.clone(), rules));
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