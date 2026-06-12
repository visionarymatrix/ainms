#[derive(Debug, Clone)]
pub struct ActiveWindow {
    pub title: String,
    pub process_name: String,
    pub process_id: i32,
}

/// Normalize a process name for consistent mapping: lowercase, strip .exe suffix.
/// This ensures "Code", "code", "CODE.exe" all become "code" so they map to
/// the same key in app_durations/app_opens HashMaps and the same DB row.
pub fn normalize_process_name(name: &str) -> String {
    let lower = name.to_lowercase();
    lower.strip_suffix(".exe").unwrap_or(&lower).to_string()
}

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub name: String,
    pub pid: i32,
    pub cmdline: String,
    pub cpu_percent: f64,
    pub memory_kb: u64,
    pub is_user_facing: bool,
}

/// Returns true if the given process name belongs to a user-facing desktop application
/// rather than a system/background process. Used to filter out noise from activity tracking.
pub fn is_desktop_app(process_name: &str) -> bool {
    let lower = process_name.to_lowercase();

    // Well-known system/background processes to skip
    const SYSTEM_PROCESSES: &[&str] = &[
        // Windows core
        "dwm", "csrss", "lsass", "smss", "winlogon", "wininit", "services",
        "svchost", "sihost", "taskhostw", "taskhost", "runtimebroker",
        "searchui", "searchhost", "shellexperiencehost", "startmenuexperiencehost",
        "applicationframehost", "applicationframe", "systemsettings",
        "backgroundtaskhost", "ctfmon", "conhost", "fontdrvhost",
        "dllhost", "wermgr", "werfault", "dashost", "dashost",
        "spoolsv", "printisolationhost", "audiodg",
        "msmpeng", "msascui", "securityhealthservice", "securityhealthsystray",
        "wmiadap", "wmiprvse", "mfeep", "hipstray",
        "tabtip", "tiptsf", "inputhost", "penmenu",
        "system", "registry", "smss", "memdiag", "splwow64",
        "mstsc", "cmd",
        // Windows services & background
        "aggregatorhost", "appactions", "ae_notifier",
        "crossdeviceservice", "crossdeviceresume",
        "dtsapo4service", "ipf_helper", "ipfsvc", "ipf_uf",
        "jhi_service", "lsaiso",
        "microsoftstartfeedprovider", "msi_terminalserver", "msi_ai_engine", "msi_centralserver", "msi_central_service", "msiservice",
        "mscopilot",
        "nvchecker", "nvcontainer", "nvsphelper64", "nvdisplay.container", "vmcompute", "vmwp", "vmmemwsl",
        "oneapp.igcc.winservice",
        "omniauthservicebroker",
        "quickshareservice",
        "rtkaudiouservice64", "rtkuwp",
        "secure system",
        "securityhealthsystray",
        "sendevsvc", "shellhost", "spoolsv",
        "unsecapp", "useroobebbroker",
        "vmms", "wslservice", "wslhost", "wslrelay",
        "widgetservice", "widgetboard", "spotifywidgetprovider",
        "wmiregistrationservice", "wmiprvse",
        "textinputhost",
        "searchfilterhost", "searchprotocolhost", "searchindexer",
        "officeclicktorun",
        "adobenotificationservice", "adobeupdateservice",
        "wudfhost",
        "monotificationux", "mousocoreworker",
        "memory compression",
        "system idle process",
        "rio",
        "taskmgr",
        "openconsole",
        // Linux system processes
        "systemd", "sshd", "cron", "atd", "rsyslogd", "syslogd",
        "dbus-daemon", "accounts-daemon", "networkmanager", "polkitd",
        "udisksd", "upowerd", "colord", "rtkit-daemon", "irqbalance",
        "snapd", "packagekitd", "avahi-daemon", "cupsd", "cups-browsed",
        "bluetoothd", "thermald", "powerd", "fwupd", "gdm",
        "pipewire", "wireplumber", "pulseaudio",
        "journald", "logind", "udevd",
        "containerd", "dockerd", "docker-proxy", "kubelet", "kube-proxy",
        "auditd", "crond", "anacron", "acpid",
        "xdg-desktop-portal", "xdg-document-portal", "xdg-permission-store",
    ];

    for sys in SYSTEM_PROCESSES {
        if lower == *sys || lower.starts_with(&format!("{}.", sys)) || lower.starts_with(&format!("{}-", sys)) {
            return false;
        }
    }

    // Known desktop applications — always count these
    const DESKTOP_APPS: &[&str] = &[
        // Browsers
        "firefox", "chrome", "msedge", "brave", "vivaldi", "opera", "safari", "iexplore",
        // IDEs & editors
        "code", "devenv", "jetbrains", "idea64", "webstorm64", "pycharm64",
        "clion64", "rider64", "goland64", "datagrip64", "rubymine64", "phpstorm64",
        "notepad", "notepad++", "sublime_text", "vim", "nvim", "emacs", "zed",
        "cursor", "windsurf",
        // Office
        "winword", "excel", "powerpnt", "onenote", "outlook", "msaccess", "mspub",
        "libreoffice", "soffice", "ooffice",
        // Communication
        "teams", "slack", "discord", "telegram", "zoom", "skype", "whatsapp",
        "signal", "viber", "wechat", "line", "thunderbird", "mail",
        // Terminal
        "windowsterminal", "powershell", "cmd", "alacritty", "kitty", "wezterm",
        "hyper", "iterm2", "terminal", "putty", "kitty",
        // Media
        "spotify", "vlc", "obs64", "obs32", "mpv", "vlc", "itunes",
        // Design & other desktop
        "figma", "sketch", "xcode", "androidstudio", "postman", "insomnia",
        "datagrip", "roblox", "steam", "epicgames",
        // File manager
        "explorer", "nautilus", "dolphin", "thunar", "nemo",
        // ChatGPT / AI tools
        "chatgpt", "copilot",
    ];

    for app in DESKTOP_APPS {
        if lower.contains(app) {
            return true;
        }
    }

    // Heuristic: processes with GUI windows are typically desktop apps.
    // If it reached here, assume it's a desktop app unless it matches obvious system patterns.
    // Processes starting with common system prefixes are likely background services.
    const SYSTEM_PREFIXES: &[&str] = &[
        "ms-", "microsoft.", "windows.", "nt ", "win", "sys",
    ];

    for prefix in SYSTEM_PREFIXES {
        if lower.starts_with(prefix) {
            return false;
        }
    }

    // Default: if it has a recognizable name, treat it as a desktop app.
    // Empty or very short names are likely system processes.
    !lower.is_empty() && lower.len() > 2
}