#[cfg(feature = "llama-cpp")]
use std::collections::HashSet;
#[cfg(feature = "llama-cpp")]
use std::sync::Arc;

use anyhow::Result;
use chrono::Utc;
use tracing::{error, info, warn};

#[cfg(feature = "llama-cpp")]
use agent_ml::agent::Agent as MlAgent;
#[cfg(feature = "llama-cpp")]
use agent_ml::llama_cpp::{LlamaCppConfig, LlamaCppProvider};
#[cfg(feature = "llama-cpp")]
use agent_ml::provider::LlmProvider;

use agent_proto::events::{ActivitySummary, BulkActivitySummaryRequest};
use agent_store::Store;



// ── Constants ───────────────────────────────────────────────────────────────

pub const ACTIVITY_BUFFER_SAMPLE_SECS: u64 = 60;
pub const ACTIVITY_BUFFER_WINDOW_SECS: u64 = 120;
pub const ACTIVITY_SUMMARY_RETENTION_SECS: i64 = 86400; // 24 hours

// ── Data types for buffered snapshots ───────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AppFocusEntry {
    pub timestamp: chrono::DateTime<Utc>,
    pub app_name: String,
    pub window_title: String,
    pub process_name: String,
}

#[derive(Debug, Clone)]
pub struct ProcessSnapshot {
    pub timestamp: chrono::DateTime<Utc>,
    pub processes: Vec<agent_collectors::ProcessInfo>,
}

// ── Activity Buffer ────────────────────────────────────────────────────────

pub struct ActivityBuffer {
    /// Screenshots captured during this window (timestamp, PNG bytes)
    screenshots: Vec<(chrono::DateTime<Utc>, Vec<u8>)>,
    /// Log of app focus changes during this window
    app_focus_log: Vec<AppFocusEntry>,
    /// Running process snapshots taken each minute
    process_snapshots: Vec<ProcessSnapshot>,
    /// App usage tracker (frequency/duration per app)
    app_usage_tracker: agent_collectors::AppUsageTracker,
    /// Start of current 5-minute window
    window_start: chrono::DateTime<Utc>,
    /// Reference to local SQLite store
    store: Store,
    /// Device ID for summaries
    device_id: String,
}

impl ActivityBuffer {
    pub fn new(store: Store, device_id: String) -> Self {
        let tracker = agent_collectors::AppUsageTracker::new(ACTIVITY_BUFFER_SAMPLE_SECS);
        Self {
            screenshots: Vec::with_capacity(5),
            app_focus_log: Vec::new(),
            process_snapshots: Vec::new(),
            app_usage_tracker: tracker,
            window_start: Utc::now(),
            store,
            device_id,
        }
    }

    /// Collect one sample: screenshot + active window + running processes.
    /// Called every ACTIVITY_BUFFER_SAMPLE_SECS (60s).
    pub async fn sample(&mut self) {
        let now = Utc::now();

        // Capture screenshot
        #[cfg(target_os = "windows")]
        if agent_screenshot::is_session_zero() {
            info!("Skipping activity buffer screenshot: running in Session 0");
        } else {
            self.capture_screenshot(now).await;
        }

        #[cfg(not(target_os = "windows"))]
        self.capture_screenshot(now).await;

        // Record active window focus — skip system/background processes
        if let Some(win) = agent_collectors::get_active_window() {
            if agent_collectors::is_desktop_app(&win.process_name) {
                self.app_focus_log.push(AppFocusEntry {
                    timestamp: now,
                    app_name: win.process_name.clone(),
                    window_title: win.title.clone(),
                    process_name: win.process_name.clone(),
                });
                self.app_usage_tracker.sample(&win);
            }
        }

        // Record running process snapshot
        let procs = agent_collectors::get_running_applications();
        self.process_snapshots.push(ProcessSnapshot {
            timestamp: now,
            processes: procs,
        });

        info!(
            screenshots = self.screenshots.len(),
            focus_entries = self.app_focus_log.len(),
            process_snaps = self.process_snapshots.len(),
            "Activity buffer sample collected"
        );
    }

    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    async fn capture_screenshot(&mut self, timestamp: chrono::DateTime<Utc>) {
        let commander = agent_screenshot::ScreenshotCommander::new();
        match commander.capture().await {
            Ok(data) => {
                info!(size = data.len(), "Activity buffer: screenshot captured");
                self.screenshots.push((timestamp, data));
            }
            Err(e) => {
                info!("Activity buffer: screenshot skipped: {}", e);
            }
        }
    }

    /// Check if the 5-minute window is complete (5 samples collected).
    pub fn should_summarize(&self) -> bool {
        let elapsed = (Utc::now() - self.window_start).num_seconds() as u64;
        elapsed >= ACTIVITY_BUFFER_WINDOW_SECS
    }

    /// Run local AI summarization on the buffered data, save summary to DB, reset buffer.
    pub async fn summarize(&mut self, role_name: &str, role_desc: &str) -> Result<()> {
        let window_end = Utc::now();
        let screenshot_count = self.screenshots.len() as u32;

        // Get app usage summary (top apps by duration)
        let usage_summary = self.app_usage_tracker.get_summary();
        let top_apps: Vec<String> = usage_summary
            .apps
            .iter()
            .take(10)
            .map(|a| {
                format!(
                    "{} ({:.0}min, {} samples)",
                    a.process_name,
                    a.duration_secs / 60.0,
                    a.sample_count
                )
            })
            .collect();

        // Try to load local AI model and generate summary
        #[cfg(feature = "llama-cpp")]
        let summary_text = match self.run_ai_summarization(role_name, role_desc).await {
            Ok(text) => text,
            Err(e) => {
                warn!("AI summarization failed, using fallback: {}", e);
                self.fallback_text_summary(role_name, role_desc)
            }
        };

        #[cfg(not(feature = "llama-cpp"))]
        let summary_text = self.fallback_text_summary(role_name, role_desc);

        // Save summary to local DB
        let summary = ActivitySummary {
            device_id: self.device_id.clone(),
            timestamp: window_end,
            window_start: self.window_start,
            window_end,
            summary_text,
            top_apps: top_apps.iter().map(|s| s.clone()).collect(),
            screenshot_count,
        };

        if let Err(e) = self.store.save_activity_summary_async(summary).await {
            error!("Failed to save activity summary to DB: {}", e);
        } else {
            info!(
                window_start = %self.window_start,
                window_end = %window_end,
                "Activity summary saved to local DB"
            );
        }

        // Reset buffer for next window
        self.reset();
        Ok(())
    }

    /// Run local AI model to generate a natural language activity summary.
    #[cfg(feature = "llama-cpp")]
    async fn run_ai_summarization(&self, role_name: &str, role_desc: &str) -> Result<String> {
        // Find GGUF model path
        let model_paths = [
            "crates/agent-ml/models/LFM2.5-VL-450M-Q4_0.gguf",
            "agent/crates/agent-ml/models/LFM2.5-VL-450M-Q4_0.gguf",
            "models/LFM2.5-VL-450M-Q4_0.gguf",
            "crates/agent-ml/models/Qwen3.5-0.8B-Q4_K_S.gguf",
            "agent/crates/agent-ml/models/Qwen3.5-0.8B-Q4_K_S.gguf",
            "models/Qwen3.5-0.8B-Q4_K_S.gguf",
        ];
        let mut chosen_path = None;
        for path in &model_paths {
            if std::path::Path::new(path).exists() {
                chosen_path = Some(path.to_string());
                break;
            }
        }

        let model_path = match chosen_path {
            Some(p) => p,
            None => anyhow::bail!("No GGUF model found at standard paths"),
        };

        info!(model_path = %model_path, "Loading GGUF model for activity summarization");

        let config = LlamaCppConfig {
            model_path: std::path::PathBuf::from(&model_path),
            mmproj_path: None,
            n_ctx: 4096,
            n_threads: 4,
            n_gpu_layers: 0,
        };

        let provider = LlamaCppProvider::new(config);
        provider.load_model(&model_path).await?;

        // Build app usage text for prompt
        let usage_summary = self.app_usage_tracker.get_summary();
        let mut app_usage_text = String::from("APP USAGE (sorted by duration):\n");
        if usage_summary.apps.is_empty() {
            app_usage_text.push_str("  No desktop apps detected in this window.\n");
        } else {
            for (i, app) in usage_summary.apps.iter().take(10).enumerate() {
                let mins = app.duration_secs / 60.0;
                let pct = if usage_summary.total_tracked_secs > 0.0 {
                    (app.duration_secs / usage_summary.total_tracked_secs) * 100.0
                } else {
                    0.0
                };
                app_usage_text.push_str(&format!(
                    "{}. {} — {:.1}min ({:.0}%) [{} samples, last title: \"{}\"]\n",
                    i + 1,
                    app.process_name,
                    mins,
                    pct,
                    app.sample_count,
                    if app.window_title.len() > 60 {
                        format!("{}...", &app.window_title[..57])
                    } else {
                        app.window_title.clone()
                    }
                ));
            }
        }

        let focus_entries_count = self.app_focus_log.len();
        let unique_apps_count = self.app_focus_log
            .iter()
            .map(|e| e.app_name.clone())
            .collect::<HashSet<_>>()
            .len();

        let agent = MlAgent::new(Arc::new(provider))
            .with_system_prompt(
                "You are an activity monitoring assistant. Analyze the provided data and write a concise 2-3 sentence summary \
                 of what the user was primarily doing during this time window. Mention the dominant application and whether \
                 the activity aligns with their assigned role. Be objective and factual."
            )
            .with_max_iterations(3);

        // Activity summarization always uses text-only (VLM reserved for compliance audit
        // in main.rs — full-resolution screenshots crash llama.cpp during VLM inference).
        let prompt = format!(
            "ACTIVITY SUMMARIZATION REQUEST:\n\
             - Employee Role: {}\n\
             - Role Work Description: {}\n\
             - Time Window: {} to {}\n\
             - Desktop Apps Detected: {} unique apps, {} focus changes\n\
             \n\
             {}\n\
             Based on the app usage data, describe in 2-3 sentences what the user was primarily doing \
             during this 5-minute window. Focus on work-related activities relevant to their role.",
            role_name,
            role_desc,
            self.window_start.format("%H:%M:%S"),
            Utc::now().format("%H:%M:%S"),
            unique_apps_count,
            focus_entries_count,
            app_usage_text,
        );

        info!("Running local Text-Only AI activity summarization...");
        let response = agent.run(prompt).await?;

        info!(response_len = response.len(), "AI activity summarization completed");
        Ok(response)
    }

    /// Fallback summary when AI is unavailable.
    fn fallback_text_summary(&self, role_name: &str, _role_desc: &str) -> String {
        let usage_summary = self.app_usage_tracker.get_summary();
        if usage_summary.apps.is_empty() {
            return format!(
                "No significant activity detected during {} to {}. Role: {}.",
                self.window_start.format("%H:%M"),
                Utc::now().format("%H:%M"),
                role_name
            );
        }

        let top_app = &usage_summary.apps[0];
        let top_app_duration_min = (top_app.duration_secs / 60.0).round() as u32;

        if usage_summary.apps.len() == 1 {
            format!(
                "User spent the entire 5-minute window in {} ({}min). Role: {}.",
                top_app.process_name, top_app_duration_min, role_name
            )
        } else {
            let second_app = &usage_summary.apps[1];
            format!(
                "User primarily used {} ({}min) and also {} during a 5-minute window. Role: {}.",
                top_app.process_name,
                top_app_duration_min,
                second_app.process_name,
                role_name
            )
        }
    }

    /// Sync pending activity summaries to the backend server.
    pub async fn sync_to_backend(
        &self,
        client: &reqwest::Client,
        server: &str,
        install_token: &str,
    ) -> Result<()> {
        let summaries = self.store.get_pending_activity_summaries_async(1000).await?;
        if summaries.is_empty() {
            info!("No pending activity summaries to sync");
            return Ok(());
        }

        let count = summaries.len();
        let bulk = BulkActivitySummaryRequest {
            device_id: self.device_id.clone(),
            summaries,
        };

        let url = format!("{}/v1/events/activity-summaries", server);
        match client
            .post(&url)
            .header("Authorization", format!("Bearer {}", install_token))
            .json(&bulk)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                info!(count, "Activity summaries synced to server");
                // Mark as uploaded
                let ids = self.store.get_pending_activity_summary_ids(1000)?;
                if !ids.is_empty() {
                    if let Err(e) = self.store.mark_activity_summaries_uploaded_async(ids).await {
                        warn!("Failed to mark activity summaries as uploaded: {}", e);
                    }
                }
                Ok(())
            }
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                warn!(status = status.as_u16(), %body, "Activity summary sync failed");
                Err(anyhow::anyhow!(
                    "Sync failed with status {}: {}",
                    status,
                    body
                ))
            }
            Err(e) => {
                warn!("Activity summary sync network error: {}", e);
                Err(anyhow::anyhow!("Network error: {}", e))
            }
        }
    }

    /// Purge activity summaries older than the retention window.
    pub async fn purge_old_summaries(&self) -> Result<usize> {
        self.store
            .purge_old_activity_summaries_async(ACTIVITY_SUMMARY_RETENTION_SECS)
            .await
    }

    /// Reset buffer for the next 5-minute window.
    fn reset(&mut self) {
        self.screenshots.clear();
        self.app_focus_log.clear();
        self.process_snapshots.clear();
        self.app_usage_tracker.reset();
        self.window_start = Utc::now();
        info!("Activity buffer reset for next 5-minute window");
    }

    /// Drain buffered screenshots and app usage data for batch upload.
    /// Returns (screenshots_with_meta, app_usage_entries).
    /// screenshots_with_meta: Vec of (timestamp, PNG bytes, app_name, window_title)
    /// app_usage_entries: Vec of (app_name, duration_secs, sample_count, window_title)
    pub fn drain_for_batch(&mut self) -> (Vec<(chrono::DateTime<Utc>, Vec<u8>, String, String)>, Vec<(String, f64, u64, String)>) {
        let usage_summary = self.app_usage_tracker.get_summary();
        let app_usage: Vec<(String, f64, u64, String)> = usage_summary
            .apps
            .iter()
            .take(10)
            .map(|a| (a.process_name.clone(), a.duration_secs, a.sample_count, a.window_title.clone()))
            .collect();

        let mut screenshots_with_meta = Vec::with_capacity(self.screenshots.len());
        for (ts, data) in self.screenshots.drain(..) {
            let (app_name, window_title) = self
                .app_focus_log
                .iter()
                .filter(|e| e.timestamp <= ts)
                .last()
                .map(|e| (e.app_name.clone(), e.window_title.clone()))
                .unwrap_or_else(|| (String::new(), String::new()));
            screenshots_with_meta.push((ts, data, app_name, window_title));
        }

        self.app_focus_log.clear();
        self.process_snapshots.clear();
        self.app_usage_tracker.reset();
        self.window_start = Utc::now();

        info!(
            screenshots = screenshots_with_meta.len(),
            app_usage_entries = app_usage.len(),
            "Activity buffer drained for batch upload"
        );

        (screenshots_with_meta, app_usage)
    }
}