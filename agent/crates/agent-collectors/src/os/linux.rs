use crate::active_window::{ActiveWindow, ProcessInfo};
use std::collections::HashMap;

const CLK_TCK: u64 = 100;

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
}

pub fn get_running_applications_with_cpu_cache(
    prev_cpu: Option<&HashMap<i32, (u64, u64)>>,
) -> Vec<ProcessInfo> {
    let mut procs = Vec::new();
    let sys_uptime_secs = read_system_uptime();

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

            procs.push(ProcessInfo {
                name,
                pid,
                cmdline,
                cpu_percent,
                memory_kb,
            });
        }
    }

    procs
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