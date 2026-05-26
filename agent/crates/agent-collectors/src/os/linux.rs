use crate::active_window::{ActiveWindow, ProcessInfo};
use crate::network::should_skip_ip;
use agent_proto::events::NetworkConnection;
use std::collections::HashMap;
use std::collections::HashSet;

const CLK_TCK: u64 = 100;

static KNOWN_DESKTOP_APPS: &[&str] = &[
    "firefox", "chrome", "chromium", "safari", "edge", "brave", "epiphany",
    "vivaldi", "opera", "waterfox", "tor-browser",
    "code", "codium", "sublime_text", "atom", "gnome-text-editor",
    "vim", "nvim", "emacs", "nano", "gedit", "kate", "mousepad",
    "terminator", "gnome-terminal", "konsole", "xterm", "alacritty",
    "kitty", "tilix", "guake", "yakuake", "wezterm", "foot",
    "nautilus", "dolphin", "thunar", "pcmanfm", "nemo", " caja",
    "libreoffice", "gimp", "inkscape", "blender", "krita", "obs",
    "vlc", "mpv", "spotify", "audacious", "rhythmbox",
    "steam", "lutris", "minecraft", "wine",
    "thunderbird", "mailspring", "geary", "evolution",
    "slack", "discord", "telegram-desktop", "teams", "zoom", "skype",
    "eog", "feh", "imv", "loupe", "nomacs",
    "evince", "okular", "atril", "zathura",
    "signal-desktop", "whatsapp-desktop", "caprine",
    "dconf-editor", "gnome-control-center", "gnome-tweaks",
    "transmission-gtk", "qbittorrent", "deluge",
    "remmina", "virt-viewer", "vinagre",
    "flatpak", "gnome-software", "software-store",
];

const KNOWN_DAEMON_PATTERNS: &[&str] = &[
    "systemd", "sshd", "cron", "atd", "rsyslogd", "syslogd",
    "dbus-daemon", "accounts-daemon", "NetworkManager", "polkitd",
    "udisksd", "upowerd", "colord", "rtkit-daemon", "irqbalance",
    "snapd", "packagekitd", "avahi-daemon", "cupsd", "cups-browsed",
    "bluetoothd", "thermald", "powerd", "fwupd", "gdm", "lightdm",
    "sddm", "xdm", "agetty", "login", "bash", "zsh", "fish",
    "sh", "dash", "ksh", "csh", "sudo", "su",
    "pulseaudio", "pipewire", "wireplumber",
    "journald", "logind", "udevd", "mountd",
    "containerd", "dockerd", "docker-proxy", "kubelet", "kube-proxy",
    "auditd", "crond", "anacron", "acpid",
];

fn is_desktop_application(name: &str, cmdline: &str, env_display: bool) -> bool {
    let lower = name.to_lowercase();
    for daemon in KNOWN_DAEMON_PATTERNS {
        if lower.contains(daemon) {
            return false;
        }
    }
    if KNOWN_DESKTOP_APPS.iter().any(|app| lower.contains(app)) {
        return true;
    }
    if lower.ends_with("-gui") || lower.ends_with("-gtk") || lower.ends_with("-qt") {
        return true;
    }
    if cmdline.contains("--gtk") || cmdline.contains("--qt") || cmdline.contains("--electron") {
        return true;
    }
    if lower.contains("electron") || lower.contains("chrome") || lower.contains("firefox") {
        return true;
    }
    if !env_display {
        return false;
    }
    if lower.starts_with("python3") || lower.starts_with("python") || lower.starts_with("node")
        || lower.starts_with("java") || lower.starts_with("cargo") || lower.starts_with("rustc")
        || lower.starts_with("go ") || lower.starts_with("make") || lower.starts_with("cmake")
    {
        return false;
    }
    true
}

pub fn get_active_window() -> Option<ActiveWindow> {
    if std::env::var("DISPLAY").is_err() {
        return None;
    }

    let pid = std::process::Command::new("xdotool")
        .arg("getactivewindow")
        .arg("getwindowpid")
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8_lossy(&o.stdout).trim().parse::<i32>().ok()
            } else {
                None
            }
        })?;

    let title = std::process::Command::new("xdotool")
        .arg("getactivewindow")
        .arg("getwindowname")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let process_name = std::fs::read_to_string(format!("/proc/{}/comm", pid))
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    Some(ActiveWindow {
        title,
        process_name,
        process_id: pid,
    })
}

pub fn get_idle_seconds() -> f64 {
    if std::env::var("DISPLAY").is_err() {
        return 0.0;
    }

    std::process::Command::new("xprintidle")
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8_lossy(&o.stdout)
                    .trim()
                    .parse::<f64>()
                    .ok()
                    .map(|ms| ms / 1000.0)
            } else {
                None
            }
        })
        .unwrap_or(0.0)
}

pub fn get_running_applications() -> Vec<ProcessInfo> {
    get_running_applications_with_cpu_cache(None)
        .into_iter()
        .filter(|p| p.is_user_facing)
        .collect()
}

pub fn get_all_running_applications() -> Vec<ProcessInfo> {
    get_running_applications_with_cpu_cache(None)
}

pub fn get_running_applications_with_cpu_cache(
    prev_cpu: Option<&HashMap<i32, (u64, u64)>>,
) -> Vec<ProcessInfo> {
    let mut procs = Vec::new();
    let sys_uptime_secs = read_system_uptime();
    let env_display = std::env::var("DISPLAY").is_ok();

    let gui_pids = get_gui_pids();

    if let Ok(entries) = std::fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let fname = entry.file_name();
            let fname_str = fname.to_string_lossy();
            if !fname_str.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            let pid: i32 = match fname_str.parse() {
                Ok(p) if p > 0 => p,
                _ => continue,
            };

            let name = match std::fs::read_to_string(format!("/proc/{}/comm", pid)) {
                Ok(s) if !s.trim().is_empty() => s.trim().to_string(),
                _ => continue,
            };

            let cmdline = std::fs::read_to_string(format!("/proc/{}/cmdline", pid))
                .unwrap_or_default()
                .replace('\0', " ")
                .trim()
                .to_string();

            let memory_kb = std::fs::read_to_string(format!("/proc/{}/status", pid))
                .ok()
                .and_then(|s| {
                    s.lines()
                        .find(|l| l.starts_with("VmRSS:"))
                        .and_then(|l| l.split_whitespace().nth(1))
                        .and_then(|v| v.parse::<u64>().ok())
                })
                .unwrap_or(0);

            let cpu_percent = match std::fs::read_to_string(format!("/proc/{}/stat", pid)) {
                Ok(stat_line) => compute_cpu_percent(&stat_line, pid, sys_uptime_secs, prev_cpu),
                Err(_) => 0.0,
            };

            let is_user_facing = gui_pids.contains(&pid)
                || is_desktop_application(&name, &cmdline, env_display);

            procs.push(ProcessInfo {
                name,
                pid,
                cmdline,
                cpu_percent,
                memory_kb,
                is_user_facing,
            });
        }
    }

    procs
}

fn get_gui_pids() -> HashSet<i32> {
    let mut gui_pids = HashSet::new();

    if let Ok(output) = std::process::Command::new("xdotool")
        .args(["search", "--onlyvisible", "--name", ""])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if let Ok(window_id) = line.trim().parse::<u64>() {
                    if let Ok(pid_output) = std::process::Command::new("xdotool")
                        .args(["getwindowpid", &window_id.to_string()])
                        .output()
                    {
                        if let Ok(pid_str) = String::from_utf8(pid_output.stdout) {
                            if let Ok(pid) = pid_str.trim().parse::<i32>() {
                                gui_pids.insert(pid);
                            }
                        }
                    }
                }
            }
        }
    }

    if let Ok(output) = std::process::Command::new("wmctrl")
        .args(["-l", "-p"])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    if let Ok(pid) = parts[2].parse::<i32>() {
                        gui_pids.insert(pid);
                    }
                }
            }
        }
    }

    gui_pids
}

fn compute_cpu_percent(
    stat_line: &str,
    pid: i32,
    sys_uptime_secs: u64,
    prev_cpu: Option<&HashMap<i32, (u64, u64)>>,
) -> f64 {
    // /proc/{pid}/stat: pid (comm) state pgrp session tty_nr ... 
    // After the closing ')', fields are 0-based relative to rest-of-line:
    //   rest[11]=utime(13), rest[12]=stime(14), rest[19]=starttime(21)
    let comm_end = match stat_line.rfind(')') {
        Some(i) => i + 1,
        None => return 0.0,
    };
    let rest = &stat_line[comm_end..];
    let fields: Vec<&str> = rest.split_whitespace().collect();

    let utime: u64 = fields.get(11).and_then(|v| v.parse().ok()).unwrap_or(0);
    let stime: u64 = fields.get(12).and_then(|v| v.parse().ok()).unwrap_or(0);
    let start_ticks: u64 = fields.get(19).and_then(|v| v.parse().ok()).unwrap_or(0);
    let total_ticks = utime + stime;

    let elapsed_secs = if sys_uptime_secs > 0 && CLK_TCK > 0 {
        let start_secs = start_ticks / CLK_TCK;
        sys_uptime_secs.saturating_sub(start_secs).max(1)
    } else {
        1
    };

    if let Some(cache) = prev_cpu {
        if let Some((prev_ticks, _)) = cache.get(&pid) {
            let delta = total_ticks.saturating_sub(*prev_ticks);
            return (delta as f64 * 100.0) / (CLK_TCK as f64 * elapsed_secs as f64);
        }
    }

    (total_ticks as f64 * 100.0) / (CLK_TCK as f64 * elapsed_secs as f64)
}

fn read_system_uptime() -> u64 {
    std::fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|s| {
            s.split_whitespace()
                .next()
                .and_then(|v| v.parse::<f64>().ok())
                .map(|f| f as u64)
        })
        .unwrap_or(0)
}

pub fn build_cpu_cache(procs: &[ProcessInfo]) -> HashMap<i32, (u64, u64)> {
    let sys_uptime_secs = read_system_uptime();
    let mut cache = HashMap::new();

    for proc_info in procs {
        if let Ok(stat_line) = std::fs::read_to_string(format!("/proc/{}/stat", proc_info.pid)) {
            let comm_end = match stat_line.rfind(')') {
                Some(i) => i + 1,
                None => continue,
            };
            let rest = &stat_line[comm_end..];
            let fields: Vec<&str> = rest.split_whitespace().collect();

            let utime: u64 = fields.get(11).and_then(|v| v.parse().ok()).unwrap_or(0);
            let stime: u64 = fields.get(12).and_then(|v| v.parse().ok()).unwrap_or(0);
            let start_ticks: u64 = fields.get(19).and_then(|v| v.parse().ok()).unwrap_or(0);
            let total_ticks = utime + stime;

            let start_secs = start_ticks / CLK_TCK;
            let elapsed = sys_uptime_secs.saturating_sub(start_secs).max(1);

            cache.insert(proc_info.pid, (total_ticks, elapsed));
        }
    }

    cache
}

// ── Network connection collection ────────────────────────────────────────────

fn parse_hex_ipv4(hex: &str) -> String {
    // /proc/net/tcp stores IPv4 as 8 hex chars in little-endian (network byte order for each 16-bit word on some kernels, but typically host byte order)
    // Format: 0100007F = 127.0.0.1. The hex is stored as little-endian u32.
    let val = match u32::from_str_radix(hex, 16) {
        Ok(v) => v,
        Err(_) => return hex.to_string(),
    };
    format!(
        "{}.{}.{}.{}",
        val & 0xFF,
        (val >> 8) & 0xFF,
        (val >> 16) & 0xFF,
        (val >> 24) & 0xFF,
    )
}

fn parse_hex_ipv6(hex: &str) -> String {
    // 32 hex chars = 128 bits, stored as 4 groups of u32 in big-endian, each u32 in host byte order
    if hex.len() != 32 {
        return hex.to_string();
    }
    let mut groups = Vec::with_capacity(8);
    for i in (0..32).step_by(4) {
        let word = match u16::from_str_radix(&hex[i..i + 4], 16) {
            Ok(v) => v.to_be_bytes(),
            Err(_) => return hex.to_string(),
        };
        groups.push(word[0]);
        groups.push(word[1]);
    }
    // Each u16 parsed from hex is already in network order, so bytes are [high, low]
    let mut ip_bytes = [0u8; 16];
    for (i, &b) in groups.iter().enumerate() {
        ip_bytes[i] = b;
    }
    std::net::Ipv6Addr::from(ip_bytes).to_string()
}

fn tcp_state_name(state_hex: &str) -> String {
    match state_hex {
        "01" => "ESTABLISHED".to_string(),
        "02" => "SYN_SENT".to_string(),
        "03" => "SYN_RECV".to_string(),
        "04" => "FIN_WAIT1".to_string(),
        "05" => "FIN_WAIT2".to_string(),
        "06" => "TIME_WAIT".to_string(),
        "07" => "CLOSE_WAIT".to_string(),
        "08" => "LAST_ACK".to_string(),
        "09" => "CLOSE_WAIT".to_string(),
        "0A" => "LISTEN".to_string(),
        _ => state_hex.to_string(),
    }
}

fn udp_state_name(state_hex: &str) -> String {
    match state_hex {
        "07" => "OPEN".to_string(),
        "01" => "ESTABLISHED".to_string(),
        _ => state_hex.to_string(),
    }
}

struct RawConnection {
    protocol: String,
    local_ip: String,
    local_port: u16,
    remote_ip: String,
    remote_port: u16,
    state: String,
    inode: u64,
}

fn parse_proc_net_line(line: &str, protocol: &str, is_ipv6: bool) -> Option<RawConnection> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 10 {
        return None;
    }

    let local = parts.get(1)?;
    let remote = parts.get(2)?;
    let state_hex = parts.get(3)?;
    let inode: u64 = parts.get(9)?.parse().ok()?;

    let (local_ip, local_port) = parse_address_port(local, is_ipv6)?;
    let (remote_ip, remote_port) = parse_address_port(remote, is_ipv6)?;

    let state = if protocol == "TCP" {
        tcp_state_name(state_hex)
    } else {
        udp_state_name(state_hex)
    };

    Some(RawConnection {
        protocol: protocol.to_string(),
        local_ip,
        local_port,
        remote_ip,
        remote_port,
        state,
        inode,
    })
}

fn parse_address_port(addr_port: &str, is_ipv6: bool) -> Option<(String, u16)> {
    let colon_pos = addr_port.rfind(':')?;
    let addr_hex = &addr_port[..colon_pos];
    let port: u16 = u16::from_str_radix(&addr_port[colon_pos + 1..], 16).ok()?;

    let ip = if is_ipv6 {
        parse_hex_ipv6(addr_hex)
    } else {
        parse_hex_ipv4(addr_hex)
    };

    Some((ip, port))
}

fn build_inode_to_pid_map() -> HashMap<u64, (i32, String)> {
    let mut map = HashMap::new();
    if let Ok(entries) = std::fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let fname = entry.file_name();
            let fname_str = fname.to_string_lossy();
            if !fname_str.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            let pid: i32 = match fname_str.parse() {
                Ok(p) if p > 0 => p,
                _ => continue,
            };

            let proc_name = std::fs::read_to_string(format!("/proc/{}/comm", pid))
                .map(|s| s.trim().to_string())
                .unwrap_or_default();

            if let Ok(fd_entries) = std::fs::read_dir(format!("/proc/{}/fd", pid)) {
                for fd_entry in fd_entries.flatten() {
                    if let Ok(link) = fd_entry.path().read_link() {
                        let link_str = link.to_string_lossy();
                        if link_str.starts_with("socket:[") && link_str.ends_with(']') {
                            let inode_str = &link_str[8..link_str.len() - 1];
                            if let Ok(inode) = inode_str.parse::<u64>() {
                                map.insert(inode, (pid, proc_name.clone()));
                            }
                        }
                    }
                }
            }
        }
    }
    map
}

fn read_proc_net_file(path: &str, protocol: &str, is_ipv6: bool) -> Vec<RawConnection> {
    let contents = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut connections = Vec::new();
    for line in contents.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(conn) = parse_proc_net_line(line, protocol, is_ipv6) {
            connections.push(conn);
        }
    }
    connections
}

pub fn get_network_connections() -> Vec<NetworkConnection> {
    let inode_map = build_inode_to_pid_map();

    let files = [
        ("/proc/net/tcp", "TCP", false),
        ("/proc/net/tcp6", "TCP", true),
        ("/proc/net/udp", "UDP", false),
        ("/proc/net/udp6", "UDP", true),
    ];

    let mut connections = Vec::new();

    for (path, protocol, is_ipv6) in files {
        let raw_conns = read_proc_net_file(path, protocol, is_ipv6);
        for raw in raw_conns {
            if should_skip_ip(&raw.remote_ip) {
                continue;
            }

            let (process_id, process_name) = inode_map
                .get(&raw.inode)
                .map(|(pid, name)| (*pid, name.clone()))
                .unwrap_or((0, String::new()));

            connections.push(NetworkConnection {
                protocol: raw.protocol,
                local_ip: raw.local_ip,
                local_port: raw.local_port,
                remote_ip: raw.remote_ip,
                remote_port: raw.remote_port,
                state: raw.state,
                process_id,
                process_name,
                remote_hostname: None,
                reconstructed_url: None,
            });
        }
    }

    connections
}

pub fn get_dns_connections() -> Vec<(i32, String)> {
    let connections = get_network_connections();
    let mut dns_pids = Vec::new();

    for conn in &connections {
        if conn.remote_port == 53 && conn.protocol == "UDP" {
            if conn.process_id > 0 {
                dns_pids.push((conn.process_id, conn.process_name.clone()));
            }
        }
    }

    dns_pids.sort_by_key(|(pid, _)| *pid);
    dns_pids.dedup_by_key(|(pid, _)| *pid);
    dns_pids
}