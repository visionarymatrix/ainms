#[cfg(target_os = "linux")]
mod linux {
    use sha2::{Digest, Sha256};

    pub fn os_type() -> String {
        "linux".to_string()
    }

    pub fn os_version() -> String {
        if let Ok(contents) = std::fs::read_to_string("/etc/os-release") {
            for line in contents.lines() {
                if line.starts_with("PRETTY_NAME=") {
                    return line
                        .trim_start_matches("PRETTY_NAME=")
                        .trim_matches('"')
                        .to_string();
                }
            }
        }
        "Linux (unknown)".to_string()
    }

    pub fn generate_fingerprint() -> String {
        let mut parts = Vec::new();

        if let Ok(contents) = std::fs::read_to_string("/etc/machine-id") {
            parts.push(contents.trim().to_string());
        }
        if let Ok(contents) = std::fs::read_to_string("/sys/class/dmi/id/product_uuid") {
            parts.push(contents.trim().to_string());
        }
        if let Ok(contents) = std::fs::read_to_string("/sys/class/dmi/id/board_serial") {
            parts.push(contents.trim().to_string());
        }
        let hostname = gethostname::gethostname()
            .into_string()
            .unwrap_or_else(|_| "unknown".to_string());
        parts.push(hostname);

        let combined = parts.join("|");
        let mut hasher = Sha256::new();
        hasher.update(combined.as_bytes());
        let result = hasher.finalize();
        format!("sha256:{:x}", result)
    }

    pub fn cpu_info() -> String {
        if let Ok(contents) = std::fs::read_to_string("/proc/cpuinfo") {
            let model_name = contents
                .lines()
                .find(|l| l.starts_with("model name"))
                .map(|l| l.split(':').nth(1).map(|s| s.trim()).unwrap_or(""))
                .unwrap_or("");
            let cores = contents.lines().filter(|l| l.starts_with("processor")).count();
            if !model_name.is_empty() {
                return format!("{} ({} cores)", model_name, cores);
            }
        }
        "Unknown CPU".to_string()
    }

    pub fn ram_info() -> String {
        if let Ok(contents) = std::fs::read_to_string("/proc/meminfo") {
            for line in contents.lines() {
                if line.starts_with("MemTotal:") {
                    let kb: u64 = line
                        .split_whitespace()
                        .nth(1)
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);
                    return format!("{} GB", kb / 1024 / 1024);
                }
            }
        }
        "Unknown RAM".to_string()
    }

    pub fn disk_info() -> String {
        let output = std::process::Command::new("df").arg("-h").arg("/").output();
        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    return format!("{} total, {} available", parts[1], parts[3]);
                }
            }
        }
        "Unknown disk".to_string()
    }

    pub fn mac_addresses() -> String {
        let mut macs = Vec::new();
        if let Ok(entries) = std::fs::read_dir("/sys/class/net") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name == "lo"
                    || name.starts_with("veth")
                    || name.starts_with("br-")
                    || name.starts_with("docker")
                    || name.starts_with("amsbr")
                    || name.starts_with("virbr")
                    || name.starts_with("vnet")
                    || name.starts_with("macvtap")
                    || name.starts_with("tun")
                    || name.starts_with("tap")
                {
                    continue;
                }
                let addr_path = format!("/sys/class/net/{}/address", name);
                if let Ok(addr) = std::fs::read_to_string(&addr_path) {
                    let addr = addr.trim().to_string();
                    if addr != "00:00:00:00:00:00" && !addr.is_empty() {
                        macs.push(format!("{}={}", name, addr));
                    }
                }
            }
        }
        macs.join(", ")
    }

    pub fn ip_addresses() -> String {
        let output = std::process::Command::new("hostname").arg("-I").output();
        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let ips: Vec<&str> = stdout
                .trim()
                .split_whitespace()
                .filter(|ip| {
                    !ip.starts_with("172.17.")
                        && !ip.starts_with("172.18.")
                        && !ip.starts_with("192.168.")
                })
                .collect();
            return ips.join(", ");
        }
        "unknown".to_string()
    }

    pub fn collect_processes() -> Vec<(String, i32, String)> {
        let mut procs = Vec::new();
        if let Ok(entries) = std::fs::read_dir("/proc") {
            for entry in entries.flatten() {
                let fname = entry.file_name();
                let fname_str = fname.to_string_lossy();
                if !fname_str.chars().all(|c| c.is_ascii_digit()) {
                    continue;
                }
                let pid: i32 = match fname_str.parse() {
                    Ok(p) => p,
                    Err(_) => continue,
                };
                let cmdline_path = format!("/proc/{}/comm", pid);
                if let Ok(name) = std::fs::read_to_string(&cmdline_path) {
                    let name = name.trim().to_string();
                    if !name.is_empty() {
                        let cmd_path = format!("/proc/{}/cmdline", pid);
                        let cmdline = std::fs::read_to_string(&cmd_path)
                            .unwrap_or_default()
                            .replace('\0', " ")
                            .trim()
                            .to_string();
                        procs.push((name, pid, cmdline));
                    }
                }
            }
        }
        procs
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use sha2::{Digest, Sha256};
    use std::process::Command;

    pub fn os_type() -> String {
        "windows".to_string()
    }

    pub fn os_version() -> String {
        let output = Command::new("cmd")
            .args(["/C", "wmic os get caption /value"])
            .output();
        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.starts_with("Caption=") {
                    return line.trim_start_matches("Caption=").trim().to_string();
                }
            }
        }
        "Windows (unknown)".to_string()
    }

    pub fn generate_fingerprint() -> String {
        let mut parts = Vec::new();

        // Windows MachineGuid from registry
        let reg_output = Command::new("reg")
            .args([
                "query",
                r"HKLM\SOFTWARE\Microsoft\Cryptography",
                "/v",
                "MachineGuid",
            ])
            .output();
        if let Ok(output) = reg_output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("MachineGuid") {
                    if let Some(val) = line.split("REG_SZ").nth(1) {
                        parts.push(val.trim().to_string());
                    }
                }
            }
        }

        // Volume serial number of C: drive
        let vol_output = Command::new("cmd")
            .args(["/C", "vol C:"])
            .output();
        if let Ok(output) = vol_output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("-") {
                    let trimmed = line.trim();
                    // Extract just the hex serial number part
                    if let Some(idx) = trimmed.rfind(' ') {
                        let serial = trimmed[idx..].trim();
                        parts.push(serial.to_string());
                    }
                }
            }
        }

        let hostname = gethostname::gethostname()
            .into_string()
            .unwrap_or_else(|_| "unknown".to_string());
        parts.push(hostname);

        let combined = parts.join("|");
        let mut hasher = Sha256::new();
        hasher.update(combined.as_bytes());
        let result = hasher.finalize();
        format!("sha256:{:x}", result)
    }

    pub fn cpu_info() -> String {
        let output = Command::new("cmd")
            .args(["/C", "wmic cpu get Name,NumberOfCores /format:list"])
            .output();
        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut name = String::new();
            let mut cores = String::new();
            for line in stdout.lines() {
                let line = line.trim();
                if line.starts_with("Name=") {
                    name = line.trim_start_matches("Name=").trim().to_string();
                } else if line.starts_with("NumberOfCores=") {
                    cores = line
                        .trim_start_matches("NumberOfCores=")
                        .trim()
                        .to_string();
                }
            }
            if !name.is_empty() {
                return format!("{} ({} cores)", name, cores);
            }
        }
        "Unknown CPU".to_string()
    }

    pub fn ram_info() -> String {
        let output = Command::new("cmd")
            .args(["/C", "wmic computersystem get TotalPhysicalMemory /value"])
            .output();
        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.starts_with("TotalPhysicalMemory=") {
                    let bytes_str = line
                        .trim_start_matches("TotalPhysicalMemory=")
                        .trim();
                    if let Ok(bytes) = bytes_str.parse::<u64>() {
                        let gb = bytes / 1024 / 1024 / 1024;
                        return format!("{} GB", gb);
                    }
                }
            }
        }
        "Unknown RAM".to_string()
    }

    pub fn disk_info() -> String {
        let output = Command::new("cmd")
            .args(["/C", "wmic logicaldisk where \"DeviceID='C:'\" get Size,FreeSpace /value"])
            .output();
        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut total_bytes: f64 = 0.0;
            let mut free_bytes: f64 = 0.0;
            for line in stdout.lines() {
                let line = line.trim();
                if line.starts_with("Size=") {
                    total_bytes = line
                        .trim_start_matches("Size=")
                        .trim()
                        .parse()
                        .unwrap_or(0.0);
                } else if line.starts_with("FreeSpace=") {
                    free_bytes = line
                        .trim_start_matches("FreeSpace=")
                        .trim()
                        .parse()
                        .unwrap_or(0.0);
                }
            }
            if total_bytes > 0.0 {
                let total_gb = total_bytes / 1024.0 / 1024.0 / 1024.0;
                let free_gb = free_bytes / 1024.0 / 1024.0 / 1024.0;
                return format!("{:.0} GB total, {:.0} GB available", total_gb, free_gb);
            }
        }
        "Unknown disk".to_string()
    }

    pub fn mac_addresses() -> String {
        let output = Command::new("cmd")
            .args(["/C", "wmic nic where \"NetEnabled=true\" get MACAddress,Name /format:list"])
            .output();
        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut macs = Vec::new();
            let mut current_name = String::new();
            let mut current_mac = String::new();
            for line in stdout.lines() {
                let line = line.trim();
                if line.starts_with("Name=") {
                    current_name = line.trim_start_matches("Name=").trim().to_string();
                } else if line.starts_with("MACAddress=") {
                    current_mac = line
                        .trim_start_matches("MACAddress=")
                        .trim()
                        .to_string();
                }
                if !current_name.is_empty() && !current_mac.is_empty() {
                    // Skip virtual/loopback adapters
                    let lower = current_name.to_lowercase();
                    if lower.contains("virtual")
                        || lower.contains("hyper-v")
                        || lower.contains("vpn")
                        || lower.contains("tunnel")
                        || lower.contains("loopback")
                        || lower.contains("bluetooth")
                    {
                        current_name.clear();
                        current_mac.clear();
                        continue;
                    }
                    macs.push(format!("{}={}", current_name, current_mac));
                    current_name.clear();
                    current_mac.clear();
                }
            }
            return macs.join(", ");
        }
        "unknown".to_string()
    }

    pub fn ip_addresses() -> String {
        let output = Command::new("cmd")
            .args(["/C", "ipconfig"])
            .output();
        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut ips = Vec::new();
            for line in stdout.lines() {
                let line = line.trim();
                if line.starts_with("IPv4 Address") || line.starts_with("IPv4-Version") {
                    if let Some(addr_part) = line.split(':').nth(1) {
                        let addr = addr_part.trim().trim_end_matches("(Preferred)");
                        if !addr.starts_with("192.168.") && !addr.starts_with("172.16.") {
                            ips.push(addr.to_string());
                        }
                    }
                }
            }
            return ips.join(", ");
        }
        "unknown".to_string()
    }

    pub fn collect_processes() -> Vec<(String, i32, String)> {
        let output = Command::new("cmd")
            .args(["/C", "tasklist /FO CSV /NH"])
            .output();
        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut procs = Vec::new();
            for line in stdout.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                // Format: "name","pid","session","session#","mem"
                let parts: Vec<&str> = line.splitn(5, ',').collect();
                if parts.len() >= 2 {
                    let name = parts[0].trim_matches('"').trim().to_string();
                    let pid_str = parts[1].trim_matches('"').trim();
                    if let Ok(pid) = pid_str.parse::<i32>() {
                        procs.push((name, pid, String::new()));
                    }
                }
            }
            return procs;
        }
        Vec::new()
    }
}

#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(target_os = "windows")]
pub use windows::*;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
mod other {
    pub fn os_type() -> String {
        "unknown".to_string()
    }

    pub fn os_version() -> String {
        "Unknown".to_string()
    }

    pub fn generate_fingerprint() -> String {
        let hostname = gethostname::gethostname()
            .into_string()
            .unwrap_or_else(|_| "unknown".to_string());
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(hostname.as_bytes());
        let result = hasher.finalize();
        format!("sha256:{:x}", result)
    }

    pub fn cpu_info() -> String {
        "Unknown CPU".to_string()
    }

    pub fn ram_info() -> String {
        "Unknown RAM".to_string()
    }

    pub fn disk_info() -> String {
        "Unknown disk".to_string()
    }

    pub fn mac_addresses() -> String {
        String::new()
    }

    pub fn ip_addresses() -> String {
        "unknown".to_string()
    }

    pub fn collect_processes() -> Vec<(String, i32, String)> {
        Vec::new()
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub use other::*;