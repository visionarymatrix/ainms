use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};
use tracing::info;

const SERVICE_NAME: &str = "ainms-agent";
const UNIT_FILE: &str = "/etc/systemd/system/ainms-agent.service";
const INSTALL_DIR: &str = "/usr/local/bin";
const BIN_NAME: &str = "ainms-agent";
const CONFIG_PATH: &str = "/etc/ainms/agent.conf";

fn installed_bin_path() -> String {
    format!("{}/{}", INSTALL_DIR, BIN_NAME)
}

fn run_chattr(files: &[&str], add: bool) -> Result<()> {
    let flag = if add { "+i" } else { "-i" };
    for f in files {
        let status = Command::new("chattr")
            .args([flag, f])
            .status()
            .context("Failed to run chattr. Is e2fsprogs installed?")?;
        if !status.success() {
            bail!("chattr {} {} failed", flag, f);
        }
    }
    Ok(())
}

fn set_immutable(files: &[&str]) -> Result<()> {
    run_chattr(files, true)
}

fn remove_immutable(files: &[&str]) -> Result<()> {
    run_chattr(files, false)
}

fn copy_binary_to_install_dir() -> Result<()> {
    let src = std::env::current_exe().context("Failed to get current executable path")?;
    let installed = installed_bin_path();
    let dst = Path::new(&installed);

    fs::create_dir_all(INSTALL_DIR)
        .with_context(|| format!("Failed to create {}", INSTALL_DIR))?;

    fs::copy(&src, dst).with_context(|| {
        format!(
            "Failed to copy binary from {} to {}",
            src.display(),
            dst.display()
        )
    })?;

    fs::set_permissions(dst, fs::Permissions::from_mode(0o755))
        .context("Failed to set binary permissions")?;

    info!("Binary installed to {}", installed_bin_path());
    Ok(())
}

fn unit_content() -> String {
    let exe = installed_bin_path();
    format!(
        "[Unit]\n\
         Description=AINMS Agent\n\
         After=network-online.target\n\
         Wants=network-online.target\n\n\
         [Service]\n\
         Type=simple\n\
         ExecStart={exe} --run-as-service --config {config}\n\
         Restart=on-failure\n\
         RestartSec=5\n\n\
         [Install]\n\
         WantedBy=multi-user.target\n",
        exe = exe,
        config = CONFIG_PATH
    )
}

pub fn install() -> Result<()> {
    copy_binary_to_install_dir()?;

    let content = unit_content();
    fs::write(UNIT_FILE, &content)
        .with_context(|| format!("Failed to write {}", UNIT_FILE))?;
    fs::set_permissions(UNIT_FILE, fs::Permissions::from_mode(0o644))?;

    let status = Command::new("systemctl")
        .args(["daemon-reload"])
        .status()
        .context("Failed to run systemctl daemon-reload")?;
    if !status.success() {
        bail!("systemctl daemon-reload failed");
    }

    let status = Command::new("systemctl")
        .args(["enable", SERVICE_NAME])
        .status()
        .context("Failed to run systemctl enable")?;
    if !status.success() {
        bail!("systemctl enable failed");
    }

    set_immutable(&[UNIT_FILE, &installed_bin_path()])?;

    info!(
        "Service '{}' installed and protected (immutable). Binary: {}, Unit: {}",
        SERVICE_NAME,
        installed_bin_path(),
        UNIT_FILE
    );
    Ok(())
}

pub fn uninstall() -> Result<()> {
    let status = Command::new("systemctl")
        .args(["stop", SERVICE_NAME])
        .status()
        .context("Failed to run systemctl stop")?;
    if !status.success() {
        info!("Service was not running (stop ignored)");
    }

    let status = Command::new("systemctl")
        .args(["disable", SERVICE_NAME])
        .status()
        .context("Failed to run systemctl disable")?;
    if !status.success() {
        info!("Service was not enabled (disable ignored)");
    }

    remove_immutable(&[UNIT_FILE, &installed_bin_path()])?;

    if Path::new(UNIT_FILE).exists() {
        fs::remove_file(UNIT_FILE)
            .with_context(|| format!("Failed to remove {}", UNIT_FILE))?;
    }

    let installed = installed_bin_path();
    if Path::new(&installed).exists() {
        fs::remove_file(&installed)
            .with_context(|| format!("Failed to remove {}", installed))?;
    }

    let status = Command::new("systemctl")
        .args(["daemon-reload"])
        .status()
        .context("Failed to run systemctl daemon-reload")?;
    if !status.success() {
        bail!("systemctl daemon-reload failed");
    }

    info!("Service '{}' uninstalled and protection removed", SERVICE_NAME);
    Ok(())
}

pub fn start() -> Result<()> {
    let status = Command::new("systemctl")
        .args(["start", SERVICE_NAME])
        .status()
        .context("Failed to run systemctl start")?;
    if !status.success() {
        bail!("systemctl start ainms-agent failed");
    }
    info!("Service '{}' started", SERVICE_NAME);
    Ok(())
}

pub fn stop() -> Result<()> {
    let status = Command::new("systemctl")
        .args(["stop", SERVICE_NAME])
        .status()
        .context("Failed to run systemctl stop")?;
    if !status.success() {
        bail!("systemctl stop ainms-agent failed");
    }
    info!("Service '{}' stopped", SERVICE_NAME);
    Ok(())
}

pub fn run_service() -> Result<()> {
    bail!("Linux service should run via systemd, not directly. Use 'systemctl start ainms-agent'");
}