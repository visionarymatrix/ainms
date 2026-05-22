pub mod platform;

pub use platform::{install, uninstall, start, stop, run_service};

#[cfg(target_os = "windows")]
pub use platform::set_agent_runner;