use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};
use tracing::info;

const SERVICE_NAME: &str = "io.ainms.agent";
const PLIST_PATH: &str = "/Library/LaunchDaemons/io.ainms.agent.plist";
const INSTALL_DIR: &str = "/usr/local/bin";
const BIN_NAME: &str = "ainms-agent";
const CONFIG_PATH: &str = "/etc/ainms/agent.conf";

fn installed_bin_path() -> String {
    format!("{}/{}", INSTALL_DIR, BIN_NAME)
}

fn run_chflags(files: &[&str], add: bool) -> Result<()> {
    let flag = if add { "schg" } else { "noschg" };
    for f in files {
        let status = Command::new("chflags")
            .args([flag, f])
            .status()
            .context("Failed to run chflags")?;
        if !status.success() {
            bail!("chflags {} {} failed", flag, f);
        }
    }
    Ok(())
}

fn set_immutable(files: &[&str]) -> Result<()> {
    run_chflags(files, true)
}

fn remove_immutable(files: &[&str]) -> Result<()> {
    run_chflags(files, false)
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

fn plist_content() -> String {
    let exe = installed_bin_path();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n\
         <dict>\n\
           <key>Label</key>\n\
           <string>{name}</string>\n\
           <key>ProgramArguments</key>\n\
           <array>\n\
             <string>{exe}</string>\n\
             <string>--run-as-service</string>\n\
             <string>--config</string>\n\
             <string>{config}</string>\n\
           </array>\n\
           <key>RunAtLoad</key>\n\
           <true/>\n\
           <key>KeepAlive</key>\n\
           <true/>\n\
           <key>StandardOutPath</key>\n\
           <string>/tmp/ainms-agent.log</string>\n\
           <key>StandardErrorPath</key>\n\
           <string>/tmp/ainms-agent.err</string>\n\
         </dict>\n\
         </plist>",
        name = SERVICE_NAME,
        exe = exe,
        config = CONFIG_PATH
    )
}

pub fn install() -> Result<()> {
    copy_binary_to_install_dir()?;

    let content = plist_content();
    fs::write(PLIST_PATH, &content)
        .with_context(|| format!("Failed to write {}", PLIST_PATH))?;
    fs::set_permissions(PLIST_PATH, fs::Permissions::from_mode(0o644))?;

    set_immutable(&[PLIST_PATH, &installed_bin_path()])?;

    let status = Command::new("launchctl")
        .args(["load", "-w", PLIST_PATH])
        .status()
        .context("Failed to run launchctl load")?;
    if !status.success() {
        bail!("launchctl load failed");
    }

    info!(
        "Service '{}' installed and protected (immutable). Binary: {}, Plist: {}",
        SERVICE_NAME,
        installed_bin_path(),
        PLIST_PATH
    );
    Ok(())
}

pub fn uninstall() -> Result<()> {
    let status = Command::new("launchctl")
        .args(["unload", "-w", PLIST_PATH])
        .status()
        .context("Failed to run launchctl unload")?;
    if !status.success() {
        info!("Service was not loaded (unload ignored)");
    }

    remove_immutable(&[PLIST_PATH, &installed_bin_path()])?;

    if Path::new(PLIST_PATH).exists() {
        fs::remove_file(PLIST_PATH)
            .with_context(|| format!("Failed to remove {}", PLIST_PATH))?;
    }

    let installed = installed_bin_path();
    if Path::new(&installed).exists() {
        fs::remove_file(&installed)
            .with_context(|| format!("Failed to remove {}", installed))?;
    }

    info!("Service '{}' uninstalled and protection removed", SERVICE_NAME);
    Ok(())
}

pub fn start() -> Result<()> {
    let status = Command::new("launchctl")
        .args(["load", "-w", PLIST_PATH])
        .status()
        .context("Failed to run launchctl load")?;
    if !status.success() {
        bail!("launchctl load failed");
    }
    info!("Service '{}' started", SERVICE_NAME);
    Ok(())
}

pub fn stop() -> Result<()> {
    let status = Command::new("launchctl")
        .args(["unload", "-w", PLIST_PATH])
        .status()
        .context("Failed to run launchctl unload")?;
    if !status.success() {
        bail!("launchctl unload failed");
    }
    info!("Service '{}' stopped", SERVICE_NAME);
    Ok(())
}

pub fn run_service() -> Result<()> {
    bail!("macOS service should run via launchd, not directly. Use 'launchctl load -w {}'", PLIST_PATH);
}