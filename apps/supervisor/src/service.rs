//! Windows Service lifecycle and Service Control Manager (SCM) integration.

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

pub const SERVICE_NAME: &str = "TrajectorySupervisor";
pub const SERVICE_DISPLAY_NAME: &str = "Trajectory Recorder Supervisor";
pub const SERVICE_DESCRIPTION: &str = "Session 0 Supervisor for Trajectory Recorder, managing crash recovery, spool watchdog, and IPC heartbeats.";

static SERVICE_CONFIG_PATH: OnceLock<PathBuf> = OnceLock::new();

#[cfg(windows)]
windows_service::define_windows_service!(ffi_service_main, supervisor_service_main);

#[cfg(windows)]
pub fn run_service(config_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    SERVICE_CONFIG_PATH
        .set(config_path)
        .map_err(|_| "Windows Service configuration was already initialized")?;
    info!(
        "Registering Windows Service dispatcher for {}...",
        SERVICE_NAME
    );
    windows_service::service_dispatcher::start(SERVICE_NAME, ffi_service_main)?;
    Ok(())
}

#[cfg(not(windows))]
pub fn run_service(_config_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    Err("Windows Service dispatcher is only supported on Windows platforms".into())
}

#[cfg(windows)]
fn supervisor_service_main(_args: Vec<OsString>) {
    use windows_service::service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    };
    use windows_service::service_control_handler::{self, ServiceControlHandlerResult};

    let cancel_token = CancellationToken::new();
    let cancel_token_clone = cancel_token.clone();

    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Stop | ServiceControl::Shutdown => {
                info!("Windows Service received STOP/SHUTDOWN control event");
                cancel_token_clone.cancel();
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    let status_handle = match service_control_handler::register(SERVICE_NAME, event_handler) {
        Ok(h) => h,
        Err(e) => {
            error!("Failed to register service control handler: {}", e);
            return;
        }
    };

    // Tell SCM that the service is running
    let running_status = ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    };

    if let Err(e) = status_handle.set_service_status(running_status) {
        error!("Failed to update service state to Running: {}", e);
        return;
    }

    // Build Tokio runtime for service execution
    let rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            error!("Failed to build Tokio runtime in service: {}", e);
            return;
        }
    };

    let config_path = match SERVICE_CONFIG_PATH.get() {
        Some(path) => path,
        None => {
            error!("Windows Service started without an explicit client configuration path");
            return;
        }
    };
    let runtime_config = match config::ClientRuntimeConfig::from_file(config_path) {
        Ok(config) => config,
        Err(error) => {
            error!(config = %config_path.display(), %error, "Failed to load client configuration");
            return;
        }
    };
    info!(
        config = %config_path.display(),
        spool = %runtime_config.spool_dir.display(),
        "Windows Service loop starting with explicit client configuration"
    );

    let result = rt.block_on(async {
        crate::run_supervisor_loop(runtime_config, config_path.to_path_buf(), cancel_token).await
    });

    if let Err(e) = result {
        error!(
            "Supervisor loop encountered error in Windows Service: {}",
            e
        );
    }

    // Report service stopped to SCM
    let stopped_status = ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    };

    let _ = status_handle.set_service_status(stopped_status);
    info!("Windows Service {} terminated cleanly.", SERVICE_NAME);
}

#[cfg(windows)]
pub fn install_service(
    exe_path: Option<PathBuf>,
    config_path: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::env;
    use windows_service::service::{
        ServiceAccess, ServiceErrorControl, ServiceInfo, ServiceStartType, ServiceType,
    };
    use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};

    let path = match exe_path {
        Some(p) => p,
        None => env::current_exe()?,
    };

    let manager =
        ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CREATE_SERVICE)?;
    let service_info = ServiceInfo {
        name: OsString::from(SERVICE_NAME),
        display_name: OsString::from(SERVICE_DISPLAY_NAME),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: path,
        launch_arguments: vec![
            OsString::from("--run-service"),
            OsString::from("--config"),
            config_path.into_os_string(),
        ],
        dependencies: vec![],
        account_name: None, // Runs as LocalSystem in Session 0
        account_password: None,
    };

    let service = manager.create_service(&service_info, ServiceAccess::CHANGE_CONFIG)?;
    service.set_description(OsString::from(SERVICE_DESCRIPTION))?;
    info!("Successfully installed Windows Service: {}", SERVICE_NAME);
    Ok(())
}

#[cfg(not(windows))]
pub fn install_service(
    _exe_path: Option<PathBuf>,
    _config_path: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("Service installation is only supported on Windows".into())
}

#[cfg(windows)]
pub fn uninstall_service() -> Result<(), Box<dyn std::error::Error>> {
    use windows_service::service::ServiceAccess;
    use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};

    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)?;
    let service = manager.open_service(SERVICE_NAME, ServiceAccess::DELETE)?;
    service.delete()?;
    info!("Successfully uninstalled Windows Service: {}", SERVICE_NAME);
    Ok(())
}

#[cfg(not(windows))]
pub fn uninstall_service() -> Result<(), Box<dyn std::error::Error>> {
    Err("Service uninstallation is only supported on Windows".into())
}
