use std::path::PathBuf;
use std::time::Duration;

use tracing::{info, warn, debug};

const CHROME_DEBUG_PORT: u16 = 9222;

pub struct ChromeLauncher {
    debug_port: u16,
}

impl ChromeLauncher {
    pub fn new() -> Self {
        Self {
            debug_port: CHROME_DEBUG_PORT,
        }
    }

    pub fn with_port(port: u16) -> Self {
        Self { debug_port: port }
    }

    pub fn find_chrome_path() -> Option<PathBuf> {
        let candidates = [
            PathBuf::from(r"C:\Program Files\Google\Chrome\Application\chrome.exe"),
            PathBuf::from(r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe"),
            PathBuf::from(
                dirs_next::data_local_dir()
                    .unwrap_or_default()
                    .join(r"Google\Chrome\Application\chrome.exe"),
            ),
        ];

        for path in &candidates {
            if path.exists() {
                return Some(path.clone());
            }
        }

        if let Ok(output) = std::process::Command::new("where")
            .arg("chrome")
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(first_line) = stdout.lines().next() {
                let path = PathBuf::from(first_line.trim());
                if path.exists() {
                    return Some(path);
                }
            }
        }

        None
    }

    pub fn is_debug_port_open(&self) -> bool {
        std::net::TcpStream::connect(format!("127.0.0.1:{}", self.debug_port)).is_ok()
    }

    pub fn launch_chrome_with_debug_port() -> Result<u32, String> {
        let chrome_path = Self::find_chrome_path()
            .ok_or("Chrome not found")?;

        let debug_port = Self::new().debug_port;

        if Self::new().is_debug_port_open() {
            info!("Chrome already running with debug port");
            return Ok(0);
        }

        let _ = Self::kill_all_chrome();
        std::thread::sleep(Duration::from_secs(3));

        // Use a separate profile directory for debug Chrome to avoid conflicts
        let debug_profile = dirs_next::data_local_dir()
            .unwrap_or_default()
            .join(r"Google\Chrome\User Data-Debug");

        let child = std::process::Command::new(&chrome_path)
            .arg(format!("--remote-debugging-port={}", debug_port))
            .arg("--no-first-run")
            .arg("--no-default-browser-check")
            .arg(format!("--user-data-dir={}", debug_profile.display()))
            .spawn()
            .map_err(|e| format!("Failed to launch Chrome: {}", e))?;

        info!(pid = child.id(), "Launched Chrome with debug port");
        Ok(child.id())
    }

    pub fn chrome_running_without_debug() -> bool {
        Self::is_chrome_running() && !Self::new().is_debug_port_open()
    }

    pub fn is_chrome_running() -> bool {
        Self::get_chrome_pids().len() > 0
    }

    pub fn get_chrome_pids() -> Vec<u32> {
        let mut pids = Vec::new();

        let output = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "Get-Process chrome -ErrorAction SilentlyContinue | Select-Object Id | ConvertTo-Json",
            ])
            .output();

        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                match json {
                    serde_json::Value::Array(ref arr) => {
                        for entry in arr {
                            if let Some(id) = entry.get("Id").and_then(|v| v.as_i64()) {
                                pids.push(id as u32);
                            }
                        }
                    }
                    serde_json::Value::Object(ref obj) => {
                        if let Some(id) = obj.get("Id").and_then(|v| v.as_i64()) {
                            pids.push(id as u32);
                        }
                    }
                    _ => {}
                }
            }
        }

        pids
    }

    pub fn kill_all_chrome() -> Result<(), String> {
        std::process::Command::new("taskkill")
            .args(["/F", "/IM", "chrome.exe"])
            .output()
            .map_err(|e| format!("Failed to kill Chrome: {}", e))?;
        std::thread::sleep(Duration::from_secs(2));
        Ok(())
    }

    pub fn restart_chrome_with_debug() -> Result<u32, String> {
        info!("Restarting Chrome with debug port enabled");
        Self::kill_all_chrome()?;
        std::thread::sleep(Duration::from_secs(1));
        Self::launch_chrome_with_debug_port()
    }

    pub fn ensure_chrome_debuggable(&self) {
        if Self::chrome_running_without_debug() {
            warn!("Chrome running without debug port, restarting");
            if let Err(e) = Self::restart_chrome_with_debug() {
                warn!(error = %e, "Failed to restart Chrome");
            }
        } else if !Self::is_chrome_running() {
            debug!("Chrome not running, launching with debug port");
            if let Err(e) = Self::launch_chrome_with_debug_port() {
                warn!(error = %e, "Failed to launch Chrome");
            }
        }
    }

    pub async fn spawn_monitor(self) {
        tokio::spawn(async move {
            loop {
                self.ensure_chrome_debuggable();
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        });
    }
}

#[cfg(target_os = "windows")]
pub mod shortcuts {
    use std::path::PathBuf;
    use tracing::{info, warn};

    pub fn find_chrome_shortcuts() -> Vec<PathBuf> {
        let mut shortcuts = Vec::new();

        if let Some(desktop) = dirs_next::desktop_dir() {
            let desktop_shortcut = desktop.join("Google Chrome.lnk");
            if desktop_shortcut.exists() {
                shortcuts.push(desktop_shortcut);
            }
        }

        let start_menu = PathBuf::from(
            r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs",
        );
        let startmenu_shortcut = start_menu.join("Google Chrome.lnk");
        if startmenu_shortcut.exists() {
            shortcuts.push(startmenu_shortcut);
        }

        if let Some(apps) = dirs_next::data_local_dir() {
            let apps_shortcut = apps
                .join(r"Microsoft\Windows\Start Menu\Programs\Google Chrome.lnk");
            if apps_shortcut.exists() {
                shortcuts.push(apps_shortcut);
            }
        }

        shortcuts
    }

    pub fn patch_shortcut_debug_port(shortcut_path: &PathBuf) -> Result<(), String> {
            use windows::Win32::System::Com::{CoCreateInstance, CoInitialize, CoUninitialize, CLSCTX_INPROC_SERVER, IPersistFile, STGM};
            use windows::Win32::UI::Shell::IShellLinkW;
            use windows::core::{Interface, PCWSTR};
            use std::ffi::OsStr;
            use std::os::windows::ffi::OsStrExt;

            const CLSID_SHELL_LINK: windows::core::GUID = windows::core::GUID::from_u128(0x00021401_0000_0000_C000_000000000046);

            unsafe {
                let _ = CoInitialize(None);

                let shell_link: IShellLinkW = CoCreateInstance(
                    &CLSID_SHELL_LINK,
                    None,
                    CLSCTX_INPROC_SERVER,
                ).map_err(|e| format!("CoCreateInstance failed: {:?}", e))?;

                let persist_file: IPersistFile = shell_link.cast().map_err(|e| format!("cast failed: {:?}", e))?;

                let path_wide: Vec<u16> = shortcut_path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
                persist_file.Load(PCWSTR(path_wide.as_ptr()), STGM(0)).map_err(|e| format!("Load failed: {:?}", e))?;

                let mut args = [0u16; 1024];
                shell_link.GetArguments(&mut args).map_err(|e| format!("GetArguments failed: {:?}", e))?;

                let current_args = String::from_utf16_lossy(&args).trim_matches('\0').to_string();

                if current_args.contains("--remote-debugging-port") {
                    info!(path = %shortcut_path.display(), "Shortcut already patched");
                    CoUninitialize();
                    return Ok(());
                }

                let debug_flag = format!("--remote-debugging-port={}", super::CHROME_DEBUG_PORT);
                let new_args = if current_args.is_empty() {
                    debug_flag.clone()
                } else {
                    format!("{} {}", current_args, debug_flag)
                };

                let new_args_wide: Vec<u16> = OsStr::new(&new_args).encode_wide().chain(std::iter::once(0)).collect();
                shell_link.SetArguments(PCWSTR(new_args_wide.as_ptr())).map_err(|e| format!("SetArguments failed: {:?}", e))?;

                persist_file.Save(None, true).map_err(|e| format!("Save failed: {:?}", e))?;

                CoUninitialize();
                info!(path = %shortcut_path.display(), "Patched shortcut with debug port");
                Ok(())
            }
        }

    pub fn patch_all_chrome_shortcuts() -> Vec<Result<(), String>> {
        let shortcuts = find_chrome_shortcuts();
        let mut results = Vec::new();

        for shortcut in shortcuts {
            info!(path = %shortcut.display(), "Patching Chrome shortcut");
            match patch_shortcut_debug_port(&shortcut) {
                Ok(()) => results.push(Ok(())),
                Err(e) => {
                    if e.contains("Access is denied") || e.contains("0x80070005") {
                        info!(path = %shortcut.display(), "Skipping system shortcut (requires admin)");
                    } else {
                        warn!(path = %shortcut.display(), error = %e, "Failed to patch shortcut");
                    }
                    results.push(Err(e));
                }
            }
        }

        results
    }

    pub fn patch_user_shortcuts_only() -> Vec<Result<(), String>> {
        let mut results = Vec::new();

        if let Some(desktop) = dirs_next::desktop_dir() {
            let desktop_shortcut = desktop.join("Google Chrome.lnk");
            if desktop_shortcut.exists() {
                info!(path = %desktop_shortcut.display(), "Patching desktop shortcut");
                results.push(patch_shortcut_debug_port(&desktop_shortcut));
            }
        }

        let local_start_menu = dirs_next::data_local_dir()
            .unwrap_or_default()
            .join(r"Microsoft\Windows\Start Menu\Programs\Google Chrome.lnk");
        if local_start_menu.exists() {
            info!(path = %local_start_menu.display(), "Patching user start menu shortcut");
            results.push(patch_shortcut_debug_port(&local_start_menu));
        }

        results
    }
}

#[cfg(not(target_os = "windows"))]
pub mod shortcuts {
    use std::path::PathBuf;

    pub fn find_chrome_shortcuts() -> Vec<PathBuf> {
        Vec::new()
    }

    pub fn patch_all_chrome_shortcuts() -> Vec<Result<(), String>> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_chrome_path() {
        let path = ChromeLauncher::find_chrome_path();
        println!("Chrome path: {:?}", path);
    }

    #[test]
    fn test_is_chrome_running() {
        let running = ChromeLauncher::is_chrome_running();
        println!("Chrome running: {}", running);
    }
}
