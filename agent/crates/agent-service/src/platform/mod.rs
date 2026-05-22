#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "windows")]
pub use windows::{install, uninstall, start, stop, run_service, set_agent_runner};

#[cfg(target_os = "macos")]
pub use macos::{install, uninstall, start, stop, run_service};

#[cfg(target_os = "linux")]
pub use linux::{install, uninstall, start, stop, run_service};