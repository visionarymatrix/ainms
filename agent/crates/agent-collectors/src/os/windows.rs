use crate::active_window::{ActiveWindow, ProcessInfo};
use agent_proto::events::NetworkConnection;

pub fn get_active_window() -> Option<ActiveWindow> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId};

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd == HWND::default() {
            return None;
        }

        let mut title_buf = [0u16; 512];
        let title_len = GetWindowTextW(hwnd, &mut title_buf);
        let title = String::from_utf16_lossy(&title_buf[..title_len as usize]);

        let mut pid: u32 = 0;
        let _ = GetWindowThreadProcessId(hwnd, Some(&mut pid));

        let process_name = get_process_name_by_pid(pid);

        Some(ActiveWindow {
            title,
            process_name,
            process_id: pid as i32,
        })
    }
}

fn get_process_name_by_pid(pid: u32) -> String {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW};
    use windows::core::PWSTR;

    unsafe {
        let process = match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            Ok(h) => h,
            Err(_) => return String::new(),
        };

        let mut name_buf = [0u16; 512];
        let mut name_size = name_buf.len() as u32;
        let result = QueryFullProcessImageNameW(
            process,
            windows::Win32::System::Threading::PROCESS_NAME_FORMAT(0),
            PWSTR(name_buf.as_mut_ptr()),
            &mut name_size,
        );

        let _ = CloseHandle(process);

        if result.is_ok() && name_size > 0 {
            let full_path = String::from_utf16_lossy(&name_buf[..name_size as usize]);
            std::path::Path::new(&full_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string()
        } else {
            String::new()
        }
    }
}

pub fn get_idle_seconds() -> f64 {
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
    use windows::Win32::System::SystemInformation::GetTickCount;

    unsafe {
        let mut lii = LASTINPUTINFO {
            cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
            dwTime: 0,
        };
        if !GetLastInputInfo(&mut lii).as_bool() {
            return 0.0;
        }
        let tick_count = GetTickCount();
        let idle_ms = tick_count.wrapping_sub(lii.dwTime);
        idle_ms as f64 / 1000.0
    }
}

pub fn get_running_applications() -> Vec<ProcessInfo> {
    get_all_running_applications()
        .into_iter()
        .filter(|p| p.is_user_facing)
        .collect()
}

pub fn get_all_running_applications() -> Vec<ProcessInfo> {
    let mut procs = Vec::new();
    let output = match std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Get-Process | Where-Object { $_.MainWindowHandle -ne 0 -or $_.Name -in @('explorer','svchost','lsass','csrss','smss','winlogon','services','spoolsv') } | Select-Object ProcessName,Id,Path,MainWindowHandle | ConvertTo-Json",
        ])
        .output()
    {
        Ok(o) => o,
        Err(_) => return procs,
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
        let entries = match json {
            serde_json::Value::Array(ref arr) => arr,
            serde_json::Value::Object(ref obj) if obj.get("ProcessName").is_some() => {
                std::slice::from_ref(&json)
            }
            _ => return procs,
        };

        for entry in entries {
            let name = entry
                .get("ProcessName")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let pid = entry
                .get("Id")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;
            let path = entry
                .get("Path")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let has_window = entry
                .get("MainWindowHandle")
                .and_then(|v| v.as_i64())
                .unwrap_or(0)
                != 0;

            if name.is_empty() || pid <= 0 {
                continue;
            }

            let is_user_facing = has_window || is_desktop_app_windows(&name);

            procs.push(ProcessInfo {
                name,
                pid,
                cmdline: path,
                cpu_percent: 0.0,
                memory_kb: 0,
                is_user_facing,
            });
        }
    }

    procs
}

fn is_desktop_app_windows(name: &str) -> bool {
    let lower = name.to_lowercase();
    let desktop_apps = [
        "firefox", "chrome", "msedge", "brave", "vivaldi", "opera",
        "code", "devenv", "jetbrains", "idea64", "webstorm64", "pycharm64",
        "clion64", "rider64", "goland64", "datagrip64",
        "winword", "excel", "powerpnt", "onenote", "outlook",
        "teams", "slack", "discord", "telegram", "zoom",
        "spotify", "vlc", "obs64", "obs32",
        "notepad", "notepad++", "windowsterminal", "cmd", "powershell",
        "explorer", "searchui", "shellexperiencehost",
    ];
    desktop_apps.iter().any(|app| lower.contains(app))
}

pub fn get_network_connections() -> Vec<NetworkConnection> {
    let mut connections = Vec::new();

    let pid_map = get_process_name_map();

    let tcp = parse_tcp_connections(&pid_map);
    connections.extend(tcp);

    let udp = parse_udp_connections(&pid_map);
    connections.extend(udp);

    connections
}

fn run_powershell(command: &str) -> Option<String> {
    std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", command])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).to_string())
            } else {
                None
            }
        })
}

fn get_process_name_map() -> std::collections::HashMap<u32, String> {
    let mut map = std::collections::HashMap::new();
    let output = match run_powershell(
        "Get-Process | Select-Object Id,ProcessName | ConvertTo-Json",
    ) {
        Some(o) => o,
        None => return map,
    };

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&output) {
        let processes = match json {
            serde_json::Value::Array(ref arr) => arr,
            serde_json::Value::Object(ref obj) if obj.contains_key("Id") => {
                std::slice::from_ref(&json)
            }
            _ => return map,
        };

        for proc in processes {
            if let (Some(id), Some(name)) = (
                proc.get("Id").and_then(|v| v.as_i64()),
                proc.get("ProcessName").and_then(|v| v.as_str()),
            ) {
                map.insert(id as u32, name.to_string());
            }
        }
    }

    map
}

fn parse_tcp_connections(
    pid_map: &std::collections::HashMap<u32, String>,
) -> Vec<NetworkConnection> {
    let mut connections = Vec::new();
    let output = match run_powershell(
        "Get-NetTCPConnection | Where-Object { $_.State -eq 'Established' -and $_.RemoteAddress -ne '0.0.0.0' -and $_.RemoteAddress -ne '::' } | Select-Object LocalAddress,LocalPort,RemoteAddress,RemotePort,State,OwningProcess | ConvertTo-Json",
    ) {
        Some(o) => o,
        None => return connections,
    };

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&output) {
        let entries = match json {
            serde_json::Value::Array(ref arr) => arr,
            serde_json::Value::Object(ref obj) if obj.contains_key("RemoteAddress") => {
                std::slice::from_ref(&json)
            }
            _ => return connections,
        };

        for entry in entries {
            let remote_ip = entry
                .get("RemoteAddress")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if should_skip_ip(remote_ip) {
                continue;
            }

            let local_ip = entry
                .get("LocalAddress")
                .and_then(|v| v.as_str())
                .unwrap_or("0.0.0.0");
            let local_port = entry
                .get("LocalPort")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u16;
            let remote_port = entry
                .get("RemotePort")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u16;
            let state = entry
                .get("State")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_lowercase();
            let pid = entry
                .get("OwningProcess")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;
            let process_name = pid_map
                .get(&(pid as u32))
                .cloned()
                .unwrap_or_default();

            connections.push(NetworkConnection {
                protocol: "tcp".to_string(),
                local_ip: local_ip.to_string(),
                local_port,
                remote_ip: remote_ip.to_string(),
                remote_port,
                state,
                process_id: pid,
                process_name,
                remote_hostname: None,
                reconstructed_url: None,
            });
        }
    }

    connections
}

fn parse_udp_connections(
    pid_map: &std::collections::HashMap<u32, String>,
) -> Vec<NetworkConnection> {
    let mut connections = Vec::new();
    let output = match run_powershell(
        "Get-NetUDPEndpoint | Where-Object { $_.RemoteAddress -ne '0.0.0.0' -and $_.RemoteAddress -ne '::' -and $_.RemoteAddress -ne '::0' } | Select-Object LocalAddress,LocalPort,RemoteAddress,RemotePort,OwningProcess | ConvertTo-Json",
    ) {
        Some(o) => o,
        None => return connections,
    };

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&output) {
        let entries = match json {
            serde_json::Value::Array(ref arr) => arr,
            serde_json::Value::Object(ref obj) if obj.contains_key("RemoteAddress") => {
                std::slice::from_ref(&json)
            }
            _ => return connections,
        };

        for entry in entries {
            let remote_ip = entry
                .get("RemoteAddress")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if should_skip_ip(remote_ip) {
                continue;
            }

            let local_ip = entry
                .get("LocalAddress")
                .and_then(|v| v.as_str())
                .unwrap_or("0.0.0.0");
            let local_port = entry
                .get("LocalPort")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u16;
            let remote_port = entry
                .get("RemotePort")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u16;
            let pid = entry
                .get("OwningProcess")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;
            let process_name = pid_map
                .get(&(pid as u32))
                .cloned()
                .unwrap_or_default();

            connections.push(NetworkConnection {
                protocol: "udp".to_string(),
                local_ip: local_ip.to_string(),
                remote_port,
                remote_ip: remote_ip.to_string(),
                local_port,
                state: "connected".to_string(),
                process_id: pid,
                process_name,
                remote_hostname: None,
                reconstructed_url: None,
            });
        }
    }

    connections
}

fn should_skip_ip(ip: &str) -> bool {
    ip.is_empty()
        || ip.starts_with("127.")
        || ip == "::1"
        || ip.starts_with("fe80:")
        || ip.starts_with("169.254.")
        || ip == "0.0.0.0"
        || ip == "::"
        || ip == "::0"
}