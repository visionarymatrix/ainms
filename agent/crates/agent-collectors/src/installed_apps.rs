use crate::active_window::is_desktop_app;

#[derive(Debug, Clone)]
pub struct InstalledApp {
    pub app_name: String,
    pub display_name: String,
    pub publisher: String,
    pub install_path: Option<String>,
}

pub fn scan_installed_apps() -> Vec<InstalledApp> {
    let mut apps: Vec<InstalledApp> = Vec::new();

    let registry_paths = [
        r"HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*",
        r"HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*",
        r"HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*",
    ];

    for reg_path in &registry_paths {
        let output = match std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "Get-ItemProperty '{}' -ErrorAction SilentlyContinue | \
                     Where-Object {{ $_.DisplayName -ne $null -and $_.DisplayName -ne '' }} | \
                     Select-Object PSChildName, DisplayName, Publisher, InstallLocation | \
                     ConvertTo-Json -Compress",
                    reg_path
                ),
            ])
            .output()
        {
            Ok(o) => o,
            Err(_) => continue,
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.trim().is_empty() {
            continue;
        }

        let entries: Vec<serde_json::Value> = match serde_json::from_str::<serde_json::Value>(&stdout) {
            Ok(serde_json::Value::Array(arr)) => arr,
            Ok(obj @ serde_json::Value::Object(_)) => vec![obj],
            _ => continue,
        };

        for entry in entries {
            let ps_child = entry
                .get("PSChildName")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let display_name = entry
                .get("DisplayName")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let publisher = entry
                .get("Publisher")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let install_path = entry
                .get("InstallLocation")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());

            if display_name.is_empty() {
                continue;
            }

            let ps_lower = ps_child.to_lowercase();

            if ps_lower.starts_with('{') || ps_lower.starts_with('_') {
                continue;
            }

            if ps_lower.contains("_microsoft.winget.source_") {
                continue;
            }

            let ps_trimmed = ps_child.trim_start_matches('_').trim_start_matches('{').trim_end_matches('}');
            if ps_trimmed.len() >= 32 && ps_trimmed.contains('-') && ps_trimmed.chars().filter(|c| *c == '-').count() >= 4 {
                continue;
            }

            let lower_pub = publisher.to_lowercase();
            if lower_pub.contains("microsoft corporation") {
                let dn_lower = display_name.to_lowercase();
                if dn_lower.contains("visual c++")
                    || dn_lower.contains("redistributable")
                    || dn_lower.contains("runtime")
                    || dn_lower.contains("debug runtime")
                    || dn_lower.contains("additional runtime")
                    || dn_lower.contains("minimum runtime")
                    || dn_lower.contains("extension sdk")
                    || dn_lower.contains("headers")
                    || dn_lower.contains("libs")
                    || dn_lower.contains("intellisense")
                    || dn_lower.contains("sdkdesktop")
                    || dn_lower.contains("signing tools")
                    || dn_lower.contains("universal crt")
                    || dn_lower.contains("windows app certification")
                    || dn_lower.contains("winappdeploy")
                    || dn_lower.contains("kits configuration")
                    || dn_lower.contains("setup configuration")
                    || dn_lower.contains("setup wmi")
                    || dn_lower.contains("tools for applications")
                    || dn_lower.contains("hosting support")
                    || dn_lower.contains("click-to-run")
                    || dn_lower.contains("update health")
                    || dn_lower.contains("shell extension")
                    || dn_lower.contains("32 bit keys")
                {
                    continue;
                }
            }

            if lower_pub.contains("nvidia corporation") {
                let dn_lower = display_name.to_lowercase();
                if ps_lower.contains("nvcontainer")
                    || ps_lower.contains("nvtelemetry")
                    || ps_lower.contains("nvdlisr")
                    || ps_lower.contains("nvpcf")
                    || ps_lower.contains("shadowplay")
                    || ps_lower.contains("nvapp.messagebus")
                    || ps_lower.contains("nvapp.nvbackend")
                    || dn_lower.contains("container")
                    || dn_lower.contains("telemetry")
                    || dn_lower.contains("runtime")
                    || dn_lower.contains("development")
                    || dn_lower.contains("documentation")
                    || dn_lower.contains("driver")
                    || dn_lower.contains("audio driver")
                    || dn_lower.contains("physx")
                    || dn_lower.contains("installer")
                    || dn_lower.contains("frameview")
                    || dn_lower.contains("messagebus")
                    || dn_lower.contains("watchdog")
                    || dn_lower.contains("nvcpl")
                    || dn_lower.contains("backend")
                    || dn_lower.contains("usbc driver")
                    || dn_lower.contains("virtual audio")
                    || dn_lower.contains("platform controllers")
                    || dn_lower.contains("cuda")
                    || dn_lower.contains("nsight")
                    || dn_lower.contains("nvjit")
                    || dn_lower.contains("nvml")
                    || dn_lower.contains("nvjpeg")
                    || dn_lower.contains("nvprune")
                    || dn_lower.contains("nvrtc")
                    || dn_lower.contains("nvtx")
                    || dn_lower.contains("nvvm")
                    || dn_lower.contains("cudart")
                    || dn_lower.contains("cublas")
                    || dn_lower.contains("cufft")
                    || dn_lower.contains("curand")
                    || dn_lower.contains("cusolver")
                    || dn_lower.contains("cusparse")
                    || dn_lower.contains("npp")
                    || dn_lower.contains("nvfatbin")
                    || dn_lower.contains("occupancy")
                    || dn_lower.contains("opencl")
                    || dn_lower.contains("sanitizer")
                    || dn_lower.contains("cuxxfilt")
                    || dn_lower.contains("cuobjdump")
                    || dn_lower.contains("cupti")
                    || dn_lower.contains("disassembler")
                    || dn_lower.contains("nvdisasm")
                    || dn_lower.contains("thrust")
                    || dn_lower.contains("libnvptxcompiler")
                    || dn_lower.contains("profiler api")
                {
                    continue;
                }
            }

            let lower_dn = display_name.to_lowercase();
            if lower_pub.contains("python software foundation") {
                if lower_dn.contains("add to path")
                    || lower_dn.contains("core interpreter")
                    || lower_dn.contains("development libraries")
                    || lower_dn.contains("documentation")
                    || lower_dn.contains("executables")
                    || lower_dn.contains("pip bootstrap")
                    || lower_dn.contains("standard library")
                    || lower_dn.contains("tcl/tk")
                    || lower_dn.contains("test suite")
                    || lower_dn.contains("launcher")
                {
                    continue;
                }
            }

            let app_name = if ps_child.is_empty() {
                display_name.to_lowercase().replace(' ', "")
            } else {
                crate::active_window::normalize_process_name(ps_child)
            };

            if !is_desktop_app(&app_name) && !is_desktop_app(&display_name.to_lowercase()) {
                continue;
            }

            if apps.iter().any(|a| a.app_name == app_name) {
                continue;
            }

            apps.push(InstalledApp {
                app_name,
                display_name: display_name.to_string(),
                publisher: publisher.to_string(),
                install_path,
            });
        }
    }

    apps.sort_by(|a, b| a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase()));
    apps
}