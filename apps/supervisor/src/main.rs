//! Trajectory Supervisor Windows Service (Session 0).
//! Manages machine identity, startup crash recovery scanning, disk pressure watchdog, and agent lifecycle.

pub mod service;

use config::{ClientRuntimeConfig, default_client_config_path};
use diagnostics::{DiagnosticsConfig, init_diagnostics};
use ipc::{IpcMessage, IpcServer};
use session::scan_and_recover_orphaned_sessions;
use spool::{
    DiskWatermarkConfig, DiskWatermarkLevel, SpoolDirectoryManager, SpoolState, evaluate_disk_level,
};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::{Child, Command};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

fn uploader_companion_path(supervisor_executable: &Path) -> Result<PathBuf, String> {
    let parent = supervisor_executable
        .parent()
        .ok_or_else(|| "supervisor executable has no parent directory".to_string())?;
    Ok(parent.join("trajectory-uploader.exe"))
}

fn validate_uploader_child_config(uploader_executable: &Path) -> Result<(), String> {
    let file_name = uploader_executable
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if !file_name.eq_ignore_ascii_case("trajectory-uploader.exe") {
        return Err("supervisor may only start its trajectory-uploader.exe companion".to_string());
    }
    Ok(())
}

fn uploader_child_arguments(config_path: &Path) -> Vec<std::ffi::OsString> {
    vec![
        std::ffi::OsString::from("--config"),
        config_path.as_os_str().to_os_string(),
    ]
}

async fn start_uploader_child(config_path: &Path) -> Result<Child, std::io::Error> {
    let supervisor_executable = std::env::current_exe()?;
    let uploader_executable =
        uploader_companion_path(&supervisor_executable).map_err(std::io::Error::other)?;
    validate_uploader_child_config(&uploader_executable).map_err(std::io::Error::other)?;
    if !uploader_executable.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "uploader companion is missing: {}",
                uploader_executable.display()
            ),
        ));
    }

    // Do not launch trajectory-agent here: Session 0 cannot observe an
    // interactive user's desktop. Its provisioning belongs to an interactive
    // logon task, while uploader is safe to run as a headless child.
    Command::new(uploader_executable)
        .env_clear()
        .args(uploader_child_arguments(config_path))
        .kill_on_drop(true)
        .spawn()
}

async fn stop_uploader_child(child: &mut Child) {
    match child.try_wait() {
        Ok(Some(status)) => info!(%status, "Uploader child had already exited"),
        Ok(None) => {
            info!("Stopping uploader child process...");
            if let Err(error) = child.kill().await {
                warn!(%error, "Failed to terminate uploader child");
            }
            if let Err(error) = child.wait().await {
                warn!(%error, "Failed to reap uploader child");
            }
        }
        Err(error) => warn!(%error, "Failed to inspect uploader child state"),
    }
}

/// Query disk space using Win32 `GetDiskFreeSpaceExW` on Windows or `sysinfo` fallback.
/// Returns `(total_bytes, total_free_bytes, free_bytes_available_to_caller)`.
pub fn get_disk_free_space<P: AsRef<Path>>(path: P) -> Result<(u64, u64, u64), std::io::Error> {
    let path = path.as_ref();

    #[cfg(windows)]
    {
        use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
        use windows::core::HSTRING;

        // Ensure path ends with slash or use canonical path for GetDiskFreeSpaceExW
        let path_str = if path.as_os_str().is_empty() {
            "C:\\"
        } else {
            path.to_str().unwrap_or("C:\\")
        };

        let path_hstring = HSTRING::from(path_str);
        let mut free_bytes_caller = 0u64;
        let mut total_bytes = 0u64;
        let mut total_free_bytes = 0u64;

        unsafe {
            let res = GetDiskFreeSpaceExW(
                &path_hstring,
                Some(&mut free_bytes_caller),
                Some(&mut total_bytes),
                Some(&mut total_free_bytes),
            );

            if let Err(e) = res {
                // If relative path fails, try default system root C:\
                let fallback_hstring = HSTRING::from("C:\\");
                GetDiskFreeSpaceExW(
                    &fallback_hstring,
                    Some(&mut free_bytes_caller),
                    Some(&mut total_bytes),
                    Some(&mut total_free_bytes),
                )
                .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
            }
        }

        Ok((total_bytes, total_free_bytes, free_bytes_caller))
    }

    #[cfg(not(windows))]
    {
        use sysinfo::Disks;
        let disks = Disks::new_with_refreshed_list();
        for disk in &disks {
            if path.starts_with(disk.mount_point()) {
                let total = disk.total_space();
                let available = disk.available_space();
                return Ok((total, available, available));
            }
        }
        // Fallback default
        Ok((
            500 * 1024 * 1024 * 1024,
            200 * 1024 * 1024 * 1024,
            200 * 1024 * 1024 * 1024,
        ))
    }
}

/// Runs the production Session 0 supervisor and its headless uploader companion.
pub async fn run_supervisor_loop(
    runtime: ClientRuntimeConfig,
    config_path: PathBuf,
    cancel_token: CancellationToken,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut uploader = start_uploader_child(&config_path).await?;
    info!(pid = ?uploader.id(), "Started headless uploader companion");

    let mut supervisor_loop = Box::pin(run_supervisor_loop_without_uploader(
        runtime.spool_dir.clone(),
        cancel_token.clone(),
    ));
    let result = tokio::select! {
        result = &mut supervisor_loop => result,
        status = uploader.wait() => {
            cancel_token.cancel();
            match status {
                Ok(status) => Err(std::io::Error::other(format!("uploader child exited unexpectedly with {status}")).into()),
                Err(error) => Err(std::io::Error::other(format!("failed to wait for uploader child: {error}")).into()),
            }
        }
    };
    stop_uploader_child(&mut uploader).await;
    result
}

/// Testable watchdog/IPC loop. Production must enter through `run_supervisor_loop`
/// so the uploader is started and monitored.
async fn run_supervisor_loop_without_uploader(
    spool_root: PathBuf,
    cancel_token: CancellationToken,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let spool_mgr = SpoolDirectoryManager::new(&spool_root)?;

    // 1. Run startup crash recovery scan
    info!("Running startup crash recovery scan on spool/recording/...");
    let recording_dir = spool_root.join("recording");
    if recording_dir.exists() {
        let recovered = scan_and_recover_orphaned_sessions(&recording_dir);
        for res in recovered {
            info!(
                "Recovered orphaned session {}: {} events restored, {} bytes corrupt tail truncated",
                res.session_id, res.recovered_events, res.bytes_truncated
            );
            let _ = spool_mgr.transition(
                &res.session_id,
                SpoolState::Recording,
                SpoolState::PendingUpload,
            );
        }
    }

    // 2. Start IPC server for agent heartbeats & alerts
    let pipe_name = r"\\.\pipe\trajectory-supervisor-ipc";
    let (ipc_tx, mut ipc_rx) = tokio::sync::mpsc::channel(100);
    let server_cancel = cancel_token.clone();
    let server = IpcServer::new(pipe_name, ipc_tx, server_cancel);
    info!("Supervisor IPC server configured on {}", pipe_name);

    tokio::spawn(async move {
        if let Err(e) = server.run().await {
            error!("IPC server encountered error: {}", e);
        }
    });

    let msg_cancel = cancel_token.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = msg_cancel.cancelled() => {
                    break;
                }
                msg = ipc_rx.recv() => {
                    match msg {
                        Some(IpcMessage::Heartbeat { queue_depth, events_captured_total, .. }) => {
                            info!("Heartbeat from agent: depth={}, events_total={}", queue_depth, events_captured_total);
                        }
                        Some(IpcMessage::DiskWatermarkAlert { disk_tier, free_bytes, total_bytes }) => {
                            warn!("Received disk watermark alert: tier={}, free={}/{}", disk_tier, free_bytes, total_bytes);
                        }
                        Some(_) => {}
                        None => break,
                    }
                }
            }
        }
    });

    // 3. Disk Pressure Watchdog loop
    let watermark_config = DiskWatermarkConfig::default();
    info!(
        "Supervisor disk watchdog loop initialized for spool {:?}",
        spool_root
    );

    loop {
        tokio::select! {
            _ = cancel_token.cancelled() => {
                info!("Supervisor loop cancellation received. Shutting down gracefully...");
                break;
            }
            _ = tokio::time::sleep(Duration::from_secs(10)) => {
                match get_disk_free_space(&spool_root) {
                    Ok((total_bytes, _total_free, available_bytes)) => {
                        let level = evaluate_disk_level(total_bytes, available_bytes, &watermark_config);

                        match level {
                            DiskWatermarkLevel::Normal => {
                                // Normal operational state
                            }
                            DiskWatermarkLevel::LowWater => {
                                warn!("Disk space LOW: available {} / total {} bytes", available_bytes, total_bytes);
                            }
                            DiskWatermarkLevel::HighWater => {
                                warn!("Disk space HIGH WATER: available {} / total {} bytes - shedding non-essential buffers", available_bytes, total_bytes);
                            }
                            DiskWatermarkLevel::Critical => {
                                error!("Disk pressure CRITICAL! Purging uploaded sessions older than 5 days...");
                                if let Err(e) = spool_mgr.purge_uploaded_older_than(5) {
                                    error!("Failed to purge uploaded sessions: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to query disk free space: {}", e);
                    }
                }
            }
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SupervisorMode {
    Console,
    InstallService,
    UninstallService,
    RunService,
    Help,
}

fn parse_supervisor_args(args: &[String]) -> Result<(SupervisorMode, PathBuf), String> {
    let mut mode = SupervisorMode::Console;
    let mut config_path = None;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--config" => {
                index += 1;
                let value = args
                    .get(index)
                    .filter(|value| !value.trim().is_empty())
                    .ok_or_else(|| "--config requires a client.env path".to_string())?;
                if config_path.replace(PathBuf::from(value)).is_some() {
                    return Err("--config may only be provided once".to_string());
                }
            }
            "--install-service" => mode = set_mode(mode, SupervisorMode::InstallService)?,
            "--uninstall-service" => mode = set_mode(mode, SupervisorMode::UninstallService)?,
            "--run-service" => mode = set_mode(mode, SupervisorMode::RunService)?,
            "--console" => mode = set_mode(mode, SupervisorMode::Console)?,
            "--help" | "-h" => mode = set_mode(mode, SupervisorMode::Help)?,
            other => return Err(format!("unknown supervisor argument: {other}")),
        }
        index += 1;
    }
    Ok((mode, config_path.unwrap_or_else(default_client_config_path)))
}

fn set_mode(current: SupervisorMode, requested: SupervisorMode) -> Result<SupervisorMode, String> {
    if current != SupervisorMode::Console && current != requested {
        return Err("only one supervisor mode may be selected".to_string());
    }
    Ok(requested)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _guard = init_diagnostics(&DiagnosticsConfig::default());

    let args: Vec<String> = std::env::args().collect();
    let (mode, config_path) = parse_supervisor_args(&args)?;
    match mode {
        SupervisorMode::Help => {
            println!("Trajectory Supervisor Options:");
            println!(
                "  --config <path>      Explicit client.env path (default: ProgramData client.env)"
            );
            println!("  --install-service    Install as Windows Service");
            println!("  --uninstall-service  Uninstall Windows Service");
            println!("  --run-service        Run under Service Control Manager");
            println!("  --console            Run in interactive console mode (default)");
            return Ok(());
        }
        SupervisorMode::UninstallService => {
            info!("Uninstalling Trajectory Supervisor Windows Service...");
            service::uninstall_service()?;
            return Ok(());
        }
        SupervisorMode::InstallService => {
            ClientRuntimeConfig::from_file(&config_path)?;
            info!(config = %config_path.display(), "Installing Trajectory Supervisor Windows Service...");
            service::install_service(None, config_path)?;
            return Ok(());
        }
        SupervisorMode::RunService => {
            info!(config = %config_path.display(), "Running as Windows Service under SCM...");
            service::run_service(config_path)?;
            return Ok(());
        }
        SupervisorMode::Console => {}
    }

    let runtime_config = ClientRuntimeConfig::from_file(&config_path)?;
    info!(config = %config_path.display(), "Starting Trajectory Supervisor in interactive console mode...");
    let cancel_token = CancellationToken::new();

    let ct_clone = cancel_token.clone();
    tokio::spawn(async move {
        if let Ok(()) = tokio::signal::ctrl_c().await {
            info!("Ctrl+C received, shutting down supervisor...");
            ct_clone.cancel();
        }
    });

    if let Err(e) = run_supervisor_loop(runtime_config, config_path, cancel_token).await {
        error!("Supervisor terminated with error: {}", e);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::ClientRuntimeConfig;

    #[test]
    fn test_get_disk_free_space_real() {
        let temp_dir = tempfile::tempdir().expect("create tempdir");
        let result = get_disk_free_space(temp_dir.path());
        assert!(
            result.is_ok(),
            "Querying disk space should succeed: {:?}",
            result.err()
        );
        let (total, total_free, available) = result.unwrap();
        assert!(total > 0, "Total disk bytes must be > 0");
        assert!(total_free > 0, "Total free disk bytes must be > 0");
        assert!(available > 0, "Available disk bytes must be > 0");
        assert!(total_free <= total, "Free bytes must be <= total bytes");
    }

    #[test]
    fn uploader_companion_must_be_a_sibling_client_executable() {
        let supervisor = Path::new(r"C:\\Program Files\\Trajectory\\trajectory-supervisor.exe");
        let uploader =
            uploader_companion_path(supervisor).expect("supervisor has a parent directory");
        assert_eq!(
            uploader,
            PathBuf::from(r"C:\\Program Files\\Trajectory\\trajectory-uploader.exe")
        );
        assert!(validate_uploader_child_config(&uploader).is_ok());
        assert!(validate_uploader_child_config(Path::new("other.exe")).is_err());
    }

    #[test]
    fn uploader_child_receives_only_an_explicit_client_config_path() {
        let config_path = Path::new(r"C:\\ProgramData\\TrajectoryRecorder\\client.env");

        assert_eq!(
            uploader_child_arguments(config_path),
            vec![
                std::ffi::OsString::from("--config"),
                config_path.as_os_str().to_os_string(),
            ]
        );
    }

    #[test]
    fn supervisor_loads_client_config_from_an_explicit_file() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("client.env");
        std::fs::write(
            &config_path,
            "DEPLOYMENT_ROLE=client\nTRAJECTORY_SERVER_URL=https://collector.example.test\nTRAJECTORY_MACHINE_ID=MACHINE-01\nTRAJECTORY_USER_ID=operator-01\nSPOOL_DIR=C:\\\\ProgramData\\\\TrajectoryRecorder\\\\spool\n",
        )
        .unwrap();

        let config = ClientRuntimeConfig::from_file(&config_path).unwrap();
        assert_eq!(config.machine_id, "MACHINE-01");
        assert_eq!(
            config.spool_dir,
            PathBuf::from(r"C:\\ProgramData\\TrajectoryRecorder\\spool")
        );
    }

    #[tokio::test]
    async fn stopping_uploader_child_reaps_the_background_process() {
        let mut child = Command::new("cmd")
            .args(["/C", "ping 127.0.0.1 -n 30 >NUL"])
            .spawn()
            .expect("start a child process");
        stop_uploader_child(&mut child).await;
        assert!(child.try_wait().unwrap().is_some());
    }

    #[tokio::test]
    async fn test_supervisor_loop_cancellation() {
        let temp_dir = tempfile::tempdir().expect("create tempdir");
        let spool_root = temp_dir.path().to_path_buf();
        let cancel_token = CancellationToken::new();

        let ct = cancel_token.clone();
        let handle =
            tokio::spawn(async move { run_supervisor_loop_without_uploader(spool_root, ct).await });

        // Let the loop start, then cancel
        tokio::time::sleep(Duration::from_millis(50)).await;
        cancel_token.cancel();

        let res = handle.await.expect("join handle");
        assert!(
            res.is_ok(),
            "Supervisor loop should terminate cleanly on cancellation"
        );
    }

    #[test]
    fn test_evaluate_disk_level_across_all_four_tiers() {
        let config = DiskWatermarkConfig::default();
        let total = 100_000_000_000u64; // 100 GB

        // Normal: 50% used
        assert_eq!(
            evaluate_disk_level(total, 50_000_000_000, &config),
            DiskWatermarkLevel::Normal
        );

        // LowWater: 72% used
        assert_eq!(
            evaluate_disk_level(total, 28_000_000_000, &config),
            DiskWatermarkLevel::LowWater
        );

        // HighWater: 86% used
        assert_eq!(
            evaluate_disk_level(total, 14_000_000_000, &config),
            DiskWatermarkLevel::HighWater
        );

        // Critical: 95% used
        assert_eq!(
            evaluate_disk_level(total, 5_000_000_000, &config),
            DiskWatermarkLevel::Critical
        );
    }

    #[test]
    fn test_get_disk_free_space_multiple_paths() {
        // Test with system drive
        let res_c = get_disk_free_space("C:\\");
        assert!(res_c.is_ok());
        let (total_c, free_c, avail_c) = res_c.unwrap();
        assert!(total_c > 0);
        assert!(free_c > 0);
        assert!(avail_c > 0);

        // Test with current directory
        let res_curr = get_disk_free_space(".");
        assert!(res_curr.is_ok());
        let (total_curr, _, _) = res_curr.unwrap();
        assert!(total_curr > 0);
    }

    #[tokio::test]
    async fn test_supervisor_startup_crash_recovery_integration() {
        let temp_dir = tempfile::tempdir().expect("create tempdir");
        let spool_root = temp_dir.path().to_path_buf();
        let spool_mgr = SpoolDirectoryManager::new(&spool_root).unwrap();

        // Create an orphaned recording session with a trailing corrupt line
        let orphan_sid = "ORPHAN_SESS_001";
        let rec_path = spool_mgr.session_path(SpoolState::Recording, orphan_sid);
        std::fs::create_dir_all(&rec_path).unwrap();
        std::fs::write(
            rec_path.join("events.raw.ndjson"),
            "{\"event\":1,\"global_event_id\":100}\n{\"event\":2,\"global_event_id\":101}\n{\"incomplete\":",
        ).unwrap();

        let cancel_token = CancellationToken::new();
        let ct = cancel_token.clone();
        let spool_root_clone = spool_root.clone();

        let handle = tokio::spawn(async move {
            run_supervisor_loop_without_uploader(spool_root_clone, ct).await
        });

        // Give the loop time to run startup crash scan
        tokio::time::sleep(Duration::from_millis(150)).await;
        cancel_token.cancel();
        let _ = handle.await;

        // Verify that the orphaned session was transitioned from recording to pending_upload
        let pending_sessions = spool_mgr.list_sessions(SpoolState::PendingUpload).unwrap();
        assert!(
            pending_sessions.contains(&orphan_sid.to_string()),
            "Orphaned session must be moved to pending_upload"
        );
    }
}
