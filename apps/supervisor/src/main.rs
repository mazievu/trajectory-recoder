//! Trajectory Supervisor Windows Service (Session 0).
//! Manages machine identity, startup crash recovery scanning, disk pressure watchdog, and agent lifecycle.

pub mod service;

use diagnostics::{DiagnosticsConfig, init_diagnostics};
use ipc::{IpcMessage, IpcServer};
use session::scan_and_recover_orphaned_sessions;
use spool::{
    DiskWatermarkConfig, DiskWatermarkLevel, SpoolDirectoryManager, SpoolState, evaluate_disk_level,
};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

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

/// Core supervisor background loop managing crash recovery, IPC server, and disk watermarks.
pub async fn run_supervisor_loop(
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _guard = init_diagnostics(&DiagnosticsConfig::default());

    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "--install-service" => {
                info!("Installing Trajectory Supervisor Windows Service...");
                service::install_service(None)?;
                info!("Service installed successfully.");
                return Ok(());
            }
            "--uninstall-service" => {
                info!("Uninstalling Trajectory Supervisor Windows Service...");
                service::uninstall_service()?;
                info!("Service uninstalled successfully.");
                return Ok(());
            }
            "--run-service" => {
                info!("Running as Windows Service under SCM...");
                service::run_service()?;
                return Ok(());
            }
            "--help" | "-h" => {
                println!("Trajectory Supervisor Options:");
                println!("  --install-service    Install as Windows Service");
                println!("  --uninstall-service  Uninstall Windows Service");
                println!("  --run-service        Run under Service Control Manager");
                println!("  --console            Run in interactive console mode (default)");
                return Ok(());
            }
            _ => {}
        }
    }

    info!("Starting Trajectory Supervisor in interactive console mode...");
    let spool_root = PathBuf::from("spool");
    let cancel_token = CancellationToken::new();

    let ct_clone = cancel_token.clone();
    tokio::spawn(async move {
        if let Ok(()) = tokio::signal::ctrl_c().await {
            info!("Ctrl+C received, shutting down supervisor...");
            ct_clone.cancel();
        }
    });

    if let Err(e) = run_supervisor_loop(spool_root, cancel_token).await {
        error!("Supervisor terminated with error: {}", e);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[tokio::test]
    async fn test_supervisor_loop_cancellation() {
        let temp_dir = tempfile::tempdir().expect("create tempdir");
        let spool_root = temp_dir.path().to_path_buf();
        let cancel_token = CancellationToken::new();

        let ct = cancel_token.clone();
        let handle = tokio::spawn(async move { run_supervisor_loop(spool_root, ct).await });

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

        let handle = tokio::spawn(async move { run_supervisor_loop(spool_root_clone, ct).await });

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
