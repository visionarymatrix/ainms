use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;

use anyhow::{Context, Result};
use tracing::{error, info, warn};

use windows_service::service::{
    ServiceAccess, ServiceControl, ServiceControlAccept, ServiceErrorControl, ServiceExitCode,
    ServiceInfo, ServiceStartType, ServiceState, ServiceStatus, ServiceType,
};
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
use windows_service::service_dispatcher;
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};

const SERVICE_NAME: &str = "AINMSAgent";
const INSTALL_DIR: &str = r"C:\Program Files\AINMS\Agent";
const BIN_NAME: &str = "agent-core.exe";
const CONFIG_PATH: &str = r"C:\ProgramData\AINMS\agent.conf";

fn installed_bin_path() -> String {
    format!(r"{}\{}", INSTALL_DIR, BIN_NAME)
}

/// Copy the current binary to the install directory, matching Linux/macOS behavior.
fn copy_binary_to_install_dir() -> Result<()> {
    let src = std::env::current_exe().context("Failed to get current executable path")?;
    let installed = installed_bin_path();
    let dst = Path::new(&installed);

    fs::create_dir_all(INSTALL_DIR)
        .with_context(|| format!("Failed to create install directory {}", INSTALL_DIR))?;

    fs::copy(&src, dst).with_context(|| {
        format!(
            "Failed to copy binary from {} to {}",
            src.display(),
            dst.display()
        )
    })?;

    info!("Binary installed to {}", installed_bin_path());
    Ok(())
}

/// Configure service recovery options via sc.exe.
///
/// The windows-service crate doesn't expose recovery actions, so we use sc.exe:
/// - First failure:  restart immediately (0s delay)
/// - Second failure: restart after 5 seconds
/// - Subsequent:     restart after 30 seconds
/// - Reset count:    after 24 hours (86400 seconds)
fn configure_service_recovery() -> Result<()> {
    let output = Command::new("sc.exe")
        .args([
            "failure",
            SERVICE_NAME,
            "reset=",
            "86400",
            "actions=",
            "restart/0/restart/5000/restart/30000",
        ])
        .output()
        .context("Failed to run sc.exe failure")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!("sc.exe failure config warning: {}", stderr.trim());
        // Non-fatal: service works without recovery config
    } else {
        info!("Service recovery configured (restart on failure)");
    }
    Ok(())
}

static AGENT_RUNNER: std::sync::OnceLock<Box<dyn Fn() + Send + Sync>> = std::sync::OnceLock::new();

pub fn set_agent_runner(runner: Box<dyn Fn() + Send + Sync>) {
    let _ = AGENT_RUNNER.set(runner);
}

pub fn install() -> Result<()> {
    // Copy binary to Program Files (same pattern as Linux/macOS)
    copy_binary_to_install_dir()?;

    let installed = installed_bin_path();
    let exe_path = Path::new(&installed).to_path_buf();

    let manager_access = ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)
        .context("Failed to connect to service manager. Run as Administrator.")?;

    let service_info = ServiceInfo {
        name: OsString::from(SERVICE_NAME),
        display_name: OsString::from("AINMS Agent"),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: exe_path,
        launch_arguments: vec![OsString::from("--run-as-service"), OsString::from("--config"), OsString::from(CONFIG_PATH)],
        dependencies: vec![],
        account_name: None,
        account_password: None,
    };

    let svc = service_manager
        .create_service(&service_info, ServiceAccess::CHANGE_CONFIG)
        .context("Failed to create service. Already installed?")?;
    svc.set_description("AINMS workplace accountability agent")?;

    // Configure auto-restart on failure (matches architecture spec)
    configure_service_recovery()?;

    info!(
        "Service '{}' installed successfully. Binary: {}, Install dir: {}",
        SERVICE_NAME,
        installed_bin_path(),
        INSTALL_DIR
    );
    Ok(())
}

pub fn uninstall() -> Result<()> {
    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)
        .context("Failed to connect to service manager. Run as Administrator.")?;

    let service_access = ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE;
    let svc = service_manager
        .open_service(SERVICE_NAME, service_access)
        .context("Service not found. Is it installed?")?;

    let status = svc.query_status()?;
    if status.current_state != ServiceState::Stopped {
        info!("Service is running, stopping...");
        svc.stop()?;
    }

    svc.delete().context("Failed to mark service for deletion")?;
    drop(svc);

    // Remove install directory and binary
    let installed = installed_bin_path();
    if Path::new(&installed).exists() {
        fs::remove_file(&installed)
            .with_context(|| format!("Failed to remove {}", installed))?;
    }
    if Path::new(INSTALL_DIR).exists() {
        // Try to remove the directory; may fail if config file still exists
        let _ = fs::remove_dir(INSTALL_DIR);
    }

    info!("Service '{}' uninstalled", SERVICE_NAME);
    Ok(())
}

pub fn start() -> Result<()> {
    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_access = ServiceAccess::START;
    let svc = service_manager
        .open_service(SERVICE_NAME, service_access)
        .context("Service not found. Is it installed?")?;

    svc.start(&[] as &[OsString])?;
    info!("Service '{}' started", SERVICE_NAME);
    Ok(())
}

pub fn stop() -> Result<()> {
    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_access = ServiceAccess::STOP | ServiceAccess::QUERY_STATUS;
    let svc = service_manager
        .open_service(SERVICE_NAME, service_access)
        .context("Service not found. Is it installed?")?;

    svc.stop()?;
    info!("Service '{}' stop requested", SERVICE_NAME);
    Ok(())
}

define_windows_service!(ffi_service_main, service_main);

fn service_main(_arguments: Vec<OsString>) {
    if let Err(e) = run_service_inner() {
        error!("Service error: {}", e);
    }
}

fn run_service_inner() -> Result<()> {
    let (stop_tx, stop_rx) = mpsc::channel::<()>();

    let event_handler = move |control_event: ServiceControl| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Stop | ServiceControl::Shutdown => {
                let _ = stop_tx.send(());
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)?;

    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::StartPending,
        controls_accepted: ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::from_secs(5),
        process_id: None,
    })?;

    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    info!("AINMS Agent service started");

    if let Some(runner) = AGENT_RUNNER.get() {
        runner();
    } else {
        error!("No agent runner registered; service will idle until stopped");
    }

    let _ = stop_rx.recv();

    info!("Service stop requested");

    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::StopPending,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::from_secs(5),
        process_id: None,
    })?;

    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    Ok(())
}

pub fn run_service() -> Result<()> {
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)?;
    Ok(())
}