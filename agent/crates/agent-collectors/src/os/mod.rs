#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::{get_active_window, get_idle_seconds, get_running_applications, get_running_applications_with_cpu_cache, build_cpu_cache, get_network_connections, get_dns_connections, get_all_running_applications};

#[cfg(target_os = "windows")]
pub use windows::{get_active_window, get_idle_seconds, get_running_applications, get_network_connections, get_all_running_applications};

#[cfg(target_os = "macos")]
pub use macos::{get_active_window, get_idle_seconds, get_running_applications, get_all_running_applications, get_network_connections};