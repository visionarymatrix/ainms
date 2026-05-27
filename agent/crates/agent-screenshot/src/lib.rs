#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
pub use linux::ScreenshotCommander;

#[cfg(target_os = "windows")]
pub use windows::ScreenshotCommander;

#[cfg(target_os = "windows")]
pub use windows::is_session_zero;

#[cfg(not(target_os = "windows"))]
pub fn is_session_zero() -> bool {
    false
}