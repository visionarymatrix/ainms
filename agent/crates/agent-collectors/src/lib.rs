pub mod active_window;
pub mod app_usage;
pub mod browser;
pub mod browser_launcher;
pub mod installed_apps;
pub mod network;
pub mod os;

pub use active_window::{ActiveWindow, ProcessInfo, is_desktop_app, normalize_process_name};
pub use app_usage::{AppUsageEntry, AppUsageTracker, UsageSummary};
pub use browser::BrowserTabMonitor;
pub use browser_launcher::{ChromeLauncher, shortcuts};
pub use installed_apps::{InstalledApp, scan_installed_apps};
pub use network::{NetworkCollector, reconstruct_url, should_skip_interface, should_skip_ip};
pub use os::*;