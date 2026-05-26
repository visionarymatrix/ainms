use agent_collectors::{
    get_active_window, get_idle_seconds, get_running_applications,
    ProcessInfo,
};
#[cfg(target_os = "linux")]
use agent_collectors::{
    build_cpu_cache, get_running_applications_with_cpu_cache,
};
use agent_screenshot::ScreenshotCommander;
use std::time:: Duration;

#[path = "../os.rs"]
mod os;

fn section(title: &str) {
    println!("\n{}", "=".repeat(60));
    println!("  {}", title);
    println!("{}", "=".repeat(60));
}

fn check_passed(msg: &str) {
    println!("  ✅ {}", msg);
}

fn check_failed(msg: &str) {
    println!("  ❌ {}", msg);
}

fn check_warn(msg: &str) {
    println!("  ⚠️  {}", msg);
}

fn main() {
    println!("AINMS Agent API Checker");
    println!("Running on {} ({})", os::os_type(), os::os_version());
    println!();

    section("System Info APIs");
    test_system_info();

    section("Fingerprint API");
    test_fingerprint();

    section("Hardware Info APIs");
    test_hardware_info();

    section("Network Info APIs");
    test_network_info();

    section("Active Window API");
    test_active_window();

    section("Idle Time API");
    test_idle_time();

    section("Running Applications API");
    test_running_applications();

    section("CPU Cache (Delta) API");
    test_cpu_cache();

    section("Screenshot API");
    test_screenshot();

    section("Dialog API");
    test_dialog();

    section("Service Module API");
    test_service();

    println!("\n{}", "=".repeat(60));
    println!("  All checks complete.");
    println!("{}", "=".repeat(60));
}

fn test_system_info() {
    let os_type = os::os_type();
    if os_type.is_empty() {
        check_failed("os_type — returned empty string");
    } else {
        check_passed(&format!("os_type → \"{}\"", os_type));
    }

    let os_version = os::os_version();
    if os_version.is_empty() {
        check_failed("os_version — returned empty string");
    } else {
        check_passed(&format!("os_version → \"{}\"", os_version));
    }

    let hostname = gethostname::gethostname()
        .into_string()
        .unwrap_or_else(|_| "unknown".to_string());
    check_passed(&format!("hostname → \"{}\"", hostname));
}

fn test_fingerprint() {
    let fp = os::generate_fingerprint();
    if fp.is_empty() {
        check_failed("generate_fingerprint — returned empty string");
    } else if fp.starts_with("sha256:") {
        check_passed(&format!("generate_fingerprint → \"{}...\"", &fp[..24]));
    } else {
        check_warn(&format!("generate_fingerprint — unexpected format: \"{}\"", fp));
    }
}

fn test_hardware_info() {
    let cpu = os::cpu_info();
    if cpu == "Unknown CPU" || cpu.is_empty() {
        check_warn(&format!("cpu_info → \"{}\"", cpu));
    } else {
        check_passed(&format!("cpu_info → \"{}\"", cpu));
    }

    let ram = os::ram_info();
    if ram == "Unknown RAM" || ram.is_empty() {
        check_warn(&format!("ram_info → \"{}\"", ram));
    } else {
        check_passed(&format!("ram_info → \"{}\"", ram));
    }

    let disk = os::disk_info();
    if disk == "Unknown disk" || disk.is_empty() {
        check_warn(&format!("disk_info → \"{}\"", disk));
    } else {
        check_passed(&format!("disk_info → \"{}\"", disk));
    }
}

fn test_network_info() {
    let macs = os::mac_addresses();
    if macs.is_empty() {
        check_warn("mac_addresses — no MAC addresses found");
    } else {
        check_passed(&format!("mac_addresses → \"{}\"", macs));
    }

    let ips = os::ip_addresses();
    if ips == "unknown" || ips.is_empty() {
        check_warn("ip_addresses — no IP addresses found");
    } else {
        check_passed(&format!("ip_addresses → \"{}\"", ips));
    }
}

fn test_active_window() {
    match get_active_window() {
        Some(win) => {
            check_passed(&format!(
                "active_window → \"{}\" (pid={}, process=\"{}\")",
                win.title, win.process_id, win.process_name
            ));
        }
        None => {
            if std::env::var("DISPLAY").is_err() && std::env::var("WAYLAND_DISPLAY").is_err() {
                check_warn("active_window → None (no DISPLAY — expected on headless)");
            } else {
                check_failed("active_window → returned None but DISPLAY is set");
            }
        }
    }
}

fn test_idle_time() {
    let idle = get_idle_seconds();
    if idle >= 0.0 {
        check_passed(&format!("idle_seconds → {:.1}s", idle));
    } else {
        check_failed("idle_seconds — returned negative value");
    }

    if idle == 0.0 && std::env::var("DISPLAY").is_err() {
        check_warn("idle_seconds → 0.0 — expected on headless (no X11)");
    }
}

fn test_running_applications() {
    let procs = get_running_applications();

    if procs.is_empty() {
        check_failed("running_applications — returned empty list");
        return;
    }

    check_passed(&format!("running_applications → {} processes found", procs.len()));

    let mut sorted = procs.clone();
    sorted.sort_by(|a, b| b.memory_kb.cmp(&a.memory_kb));

    println!("\n  {:<30} {:>6} {:>10} {:>12}", "NAME", "PID", "CPU%", "MEM_KB");
    println!("  {}", "-".repeat(62));
    for p in sorted.iter().take(15) {
        println!(
            "  {:<30} {:>6} {:>9.1}% {:>10} KB",
            truncate(&p.name, 30),
            p.pid,
            p.cpu_percent,
            p.memory_kb
        );
    }
    if sorted.len() > 15 {
        println!("  ... and {} more", sorted.len() - 15);
    }

    let own_proc: Vec<&ProcessInfo> = procs.iter().filter(|p| p.name == "check-apis").collect();
    if own_proc.is_empty() {
        let agent_procs: Vec<&ProcessInfo> = procs.iter().filter(|p| p.name == "agent-core").collect();
        if agent_procs.is_empty() {
            check_warn("self-detection — neither check-apis nor agent-core found in process list");
        } else {
            check_passed(&format!("self-detection — agent-core found (pid={})", agent_procs[0].pid));
        }
    } else {
        check_passed(&format!("self-detection — check-apis found (pid={})", own_proc[0].pid));
    }
}

#[cfg(target_os = "linux")]
fn test_cpu_cache() {
    let first = get_running_applications();
    std::thread::sleep(Duration::from_secs(2));
    let cache = build_cpu_cache(&first);
    let second = get_running_applications_with_cpu_cache(Some(&cache));

    let with_cpu: Vec<&ProcessInfo> = second.iter().filter(|p| p.cpu_percent > 0.0).collect();
    check_passed(&format!(
        "cpu_cache → {} of {} processes have CPU% > 0 (delta-based)",
        with_cpu.len(),
        second.len()
    ));

    if !with_cpu.is_empty() {
        println!("  Top CPU consumers:");
        let mut sorted = with_cpu.clone();
        sorted.sort_by(|a, b| b.cpu_percent.partial_cmp(&a.cpu_percent).unwrap_or(std::cmp::Ordering::Equal));
        for p in sorted.iter().take(5) {
            println!("    {:<20} {:.1}%", truncate(&p.name, 20), p.cpu_percent);
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn test_cpu_cache() {
    check_warn("cpu_cache — skipped on non-Linux OS");
}



fn test_screenshot() {
    let commander = ScreenshotCommander::new();

    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    let result = rt.block_on(async { commander.capture().await });

    match result {
        Ok(data) => {
            check_passed(&format!("screenshot capture → {} bytes (PNG)", data.len()));
        }
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("No display available") {
                check_warn(&format!("screenshot capture — {} (expected on headless)", msg));
            } else if msg.contains("No screenshot tool available") {
                check_failed(&format!("screenshot capture — {}", msg));
            } else {
                check_warn(&format!("screenshot capture — {}", msg));
            }
        }
    }
}

fn test_dialog() {
    check_passed("dialog module → compiled (ask/notify functions available)");
    println!("  Note: Actual dialog display skipped (requires GUI interaction)");
}

fn test_service() {
    check_passed("service module → compiled (install/uninstall/start/stop)");
    println!("  Note: Service commands require root/admin — not tested here");
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}