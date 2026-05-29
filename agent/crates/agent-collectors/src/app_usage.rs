use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::{DateTime, Local};
use serde::Serialize;

use crate::ActiveWindow;

#[derive(Debug, Clone, Serialize)]
pub struct AppUsageEntry {
    pub process_name: String,
    pub window_title: String,
    pub duration_secs: f64,
    pub last_seen: DateTime<Local>,
    pub sample_count: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageSummary {
    pub total_tracked_secs: f64,
    pub apps: Vec<AppUsageEntry>,
    pub session_start: DateTime<Local>,
    pub session_end: DateTime<Local>,
}

#[derive(Debug)]
pub struct AppUsageTracker {
    state: Arc<Mutex<TrackerState>>,
    sample_interval: Duration,
}

#[derive(Debug)]
struct TrackerState {
    current_app: Option<String>,
    current_title: String,
    current_pid: i32,
    last_sample_time: Instant,
    session_start: DateTime<Local>,
    usage: HashMap<String, AppUsageEntry>,
    is_running: bool,
}

impl AppUsageTracker {
    pub fn new(sample_interval_secs: u64) -> Self {
        Self {
            state: Arc::new(Mutex::new(TrackerState {
                current_app: None,
                current_title: String::new(),
                current_pid: 0,
                last_sample_time: Instant::now(),
                session_start: Local::now(),
                usage: HashMap::new(),
                is_running: false,
            })),
            sample_interval: Duration::from_secs(sample_interval_secs),
        }
    }

    pub fn with_default_interval() -> Self {
        Self::new(5)
    }

    pub fn start(&self) {
        let mut state = self.state.lock().unwrap();
        state.is_running = true;
        state.session_start = Local::now();
        state.last_sample_time = Instant::now();
    }

    pub fn stop(&self) {
        let mut state = self.state.lock().unwrap();
        state.is_running = false;
        self.flush_current(&mut state);
    }

    pub fn sample(&self, window: &ActiveWindow) {
        let mut state = self.state.lock().unwrap();
        if !state.is_running {
            return;
        }

        let now = Instant::now();
        let elapsed = now.duration_since(state.last_sample_time).as_secs_f64();
        state.last_sample_time = now;

        let same_app = state
            .current_app
            .as_ref()
            .map(|a| a == &window.process_name)
            .unwrap_or(false);

        if same_app {
            if let Some(entry) = state.usage.get_mut(&window.process_name) {
                entry.duration_secs += elapsed;
                entry.sample_count += 1;
                entry.last_seen = Local::now();
                entry.window_title = window.title.clone();
            }
        } else {
            self.flush_current(&mut state);
            self.record_new_app(&mut state, window);
        }
    }

    fn record_new_app(&self, state: &mut TrackerState, window: &ActiveWindow) {
        let now = Instant::now();
        let elapsed = now.duration_since(state.last_sample_time).as_secs_f64();
        state.last_sample_time = now;

        let entry = state
            .usage
            .entry(window.process_name.clone())
            .or_insert_with(|| AppUsageEntry {
                process_name: window.process_name.clone(),
                window_title: window.title.clone(),
                duration_secs: 0.0,
                last_seen: Local::now(),
                sample_count: 0,
            });

        entry.duration_secs += elapsed;
        entry.window_title = window.title.clone();
        entry.last_seen = Local::now();
        entry.sample_count += 1;

        state.current_app = Some(window.process_name.clone());
        state.current_title = window.title.clone();
        state.current_pid = window.process_id;
    }

    fn flush_current(&self, state: &mut TrackerState) {
        if let Some(ref app) = state.current_app {
            let now = Instant::now();
            let elapsed = now.duration_since(state.last_sample_time).as_secs_f64();

            if let Some(entry) = state.usage.get_mut(app) {
                entry.duration_secs += elapsed;
                entry.last_seen = Local::now();
            }

            state.last_sample_time = now;
        }
    }

    pub fn get_summary(&self) -> UsageSummary {
        let state = self.state.lock().unwrap();
        let mut apps: Vec<AppUsageEntry> = state.usage.values().cloned().collect();
        apps.sort_by(|a, b| b.duration_secs.partial_cmp(&a.duration_secs).unwrap());

        UsageSummary {
            total_tracked_secs: apps.iter().map(|a| a.duration_secs).sum(),
            apps,
            session_start: state.session_start,
            session_end: Local::now(),
        }
    }

    pub fn get_current_app(&self) -> Option<String> {
        self.state.lock().unwrap().current_app.clone()
    }

    pub fn get_usage_for_app(&self, process_name: &str) -> Option<AppUsageEntry> {
        self.state.lock().unwrap().usage.get(process_name).cloned()
    }

    pub fn reset(&self) {
        let mut state = self.state.lock().unwrap();
        state.usage.clear();
        state.current_app = None;
        state.session_start = Local::now();
        state.last_sample_time = Instant::now();
    }

    pub fn is_running(&self) -> bool {
        self.state.lock().unwrap().is_running
    }

    pub fn spawn_sampler(self) -> Arc<Self> {
        let tracker = Arc::new(self);
        let tracker_clone = Arc::clone(&tracker);

        tracker_clone.start();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tracker_clone.sample_interval).await;

                if !tracker_clone.is_running() {
                    break;
                }

                if let Some(window) = crate::os::get_active_window() {
                    tracker_clone.sample(&window);
                }
            }
        });

        tracker
    }
}

impl std::fmt::Display for UsageSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== Application Usage Summary ===")?;
        writeln!(
            f,
            "Session: {} to {}",
            self.session_start.format("%Y-%m-%d %H:%M:%S"),
            self.session_end.format("%Y-%m-%d %H:%M:%S")
        )?;
        writeln!(
            f,
            "Total tracked time: {:.1} seconds",
            self.total_tracked_secs
        )?;
        writeln!(f)?;

        for (i, app) in self.apps.iter().enumerate().take(20) {
            let pct = if self.total_tracked_secs > 0.0 {
                (app.duration_secs / self.total_tracked_secs) * 100.0
            } else {
                0.0
            };
            writeln!(
                f,
                "{:>2}. {:<25} {:>8.1}s ({:>5.1}%) [{} samples]",
                i + 1,
                app.process_name,
                app.duration_secs,
                pct,
                app.sample_count
            )?;
        }

        Ok(())
    }
}
