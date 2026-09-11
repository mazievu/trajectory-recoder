//! Trajectory Desktop Capture Agent binary.
//! Integrates Win32 hooks, window tracking, clipboard, file events, UIA, privacy filtering,
//! correlation engine, NDJSON & SQLite WAL persistence, and Named Pipe IPC server.

use clipboard_win::ClipboardManager;
use config::{ClientRuntimeConfig, default_client_config_path};
use core_types::event::RawEventPayload;
use core_types::metadata::TargetMetadata;
use correlator::CorrelationEngine;
use diagnostics::{DiagnosticsConfig, init_diagnostics};
use event_bus::bus::{EventBus, EventBusConfig};
use file_events_win::FileWatcherManager;
use input_win::manager::InputHookManager;
use ipc::{IpcMessage, IpcServer};
use privacy::engine::{PrivacyEngine, PrivacyPolicy};
use session::manager::SessionManager;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tracing::{error, info, warn};
use uia_win::inspector::UiaInspector;
use window_win::tracker::WindowTracker;

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeIdentity {
    machine_id: String,
    user_id: String,
}

impl RuntimeIdentity {
    fn from_config(config: &ClientRuntimeConfig) -> Self {
        Self {
            machine_id: config.machine_id.clone(),
            user_id: config.user_id.clone(),
        }
    }

    fn from_values(machine_id: Option<String>, user_id: Option<String>) -> Result<Self, String> {
        let machine_id = machine_id
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                "TRAJECTORY_MACHINE_ID is required; refusing an un-enrolled capture client"
                    .to_string()
            })?;
        let user_id = user_id
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                "TRAJECTORY_USER_ID is required; refusing to record an unidentified user"
                    .to_string()
            })?;
        Ok(Self {
            machine_id,
            user_id,
        })
    }
}

#[derive(Debug, Clone, Default)]
struct CaptureCliOptions {
    config_path: Option<PathBuf>,
    raw_log_console: bool,
    show_help: bool,
}

fn parse_cli_options(args: &[String]) -> Result<CaptureCliOptions, String> {
    let mut options = CaptureCliOptions::default();
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--config" => {
                index += 1;
                let value = args
                    .get(index)
                    .filter(|value| !value.trim().is_empty())
                    .ok_or_else(|| "--config requires a client.env path".to_string())?;
                if options.config_path.replace(PathBuf::from(value)).is_some() {
                    return Err("--config may only be provided once".to_string());
                }
            }
            "--raw-log" | "-r" => {
                options.raw_log_console = true;
            }
            "--help" | "-h" => {
                options.show_help = true;
            }
            other => return Err(format!("unknown capture-agent argument: {other}")),
        }
        index += 1;
    }
    Ok(options)
}

fn resolve_config_path(explicit_path: Option<PathBuf>) -> PathBuf {
    let resolved = if let Some(path) = explicit_path {
        path
    } else if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("client.env");
            if candidate.is_file() {
                candidate
            } else if let Ok(cwd) = std::env::current_dir() {
                let candidate = cwd.join("client.env");
                if candidate.is_file() {
                    candidate
                } else {
                    default_client_config_path()
                }
            } else {
                default_client_config_path()
            }
        } else {
            default_client_config_path()
        }
    } else {
        default_client_config_path()
    };
    std::path::absolute(&resolved).unwrap_or(resolved)
}

fn spawn_uploader_companion(config_path: &Path) -> Option<tokio::process::Child> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let uploader = dir.join("trajectory-uploader.exe");
    if !uploader.is_file() {
        return None;
    }
    info!("Starting background uploader companion: {}", uploader.display());
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let mut cmd = tokio::process::Command::new(uploader);
        cmd.arg("--config").arg(config_path);
        cmd.current_dir(dir);
        cmd.creation_flags(CREATE_NO_WINDOW);
        cmd.kill_on_drop(true);
        cmd.spawn().ok()
    }
    #[cfg(not(windows))]
    {
        let mut cmd = tokio::process::Command::new(uploader);
        cmd.arg("--config").arg(config_path);
        cmd.current_dir(dir);
        cmd.kill_on_drop(true);
        cmd.spawn().ok()
    }
}

fn capture_config_path(args: &[String]) -> Result<PathBuf, String> {
    let opts = parse_cli_options(args)?;
    if opts.show_help {
        return Err("Usage: trajectory-agent [--config C:\\ProgramData\\TrajectoryRecorder\\client.env] [--raw-log]".to_string());
    }
    Ok(resolve_config_path(opts.config_path))
}

/// The only events that warrant an expensive UI Automation lookup.
/// Mouse movement is intentionally excluded: it is transport noise, not a
/// user action. Keyboard and foreground events use the focused element because
/// they do not carry a screen coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UiaLookupRequest {
    Point(i32, i32),
    Focused,
}

fn uia_lookup_request(payload: &RawEventPayload) -> Option<UiaLookupRequest> {
    match payload {
        RawEventPayload::Mouse(mouse)
            if matches!(
                mouse.event_type.as_str(),
                "MOUSE_DOWN" | "MOUSE_UP" | "CLICK" | "DOUBLE_CLICK" | "MOUSE_WHEEL"
            ) =>
        {
            Some(UiaLookupRequest::Point(mouse.physical_x, mouse.physical_y))
        }
        RawEventPayload::Keyboard(keyboard) if keyboard.event_type == "KEY_DOWN" => {
            Some(UiaLookupRequest::Focused)
        }
        RawEventPayload::Window(window) if window.event_type == "FOREGROUND" => {
            Some(UiaLookupRequest::Focused)
        }
        _ => None,
    }
}

async fn target_metadata_for_event(
    inspector: &UiaInspector,
    payload: &RawEventPayload,
) -> Option<TargetMetadata> {
    match uia_lookup_request(payload) {
        Some(UiaLookupRequest::Point(x, y)) => inspector.inspect_point(x, y).await,
        Some(UiaLookupRequest::Focused) => inspector.inspect_focused().await,
        None => None,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let cli_opts = match parse_cli_options(&args) {
        Ok(opts) => opts,
        Err(err) => {
            eprintln!("Error: {err}");
            eprintln!("Usage: trajectory-agent [--config <path>] [--raw-log]");
            std::process::exit(1);
        }
    };

    if cli_opts.show_help {
        println!("Trajectory Desktop Capture Agent (Edition 2024)");
        println!("Usage: trajectory-agent [OPTIONS]");
        println!("Options:");
        println!("  --config <path>    Path to client.env configuration file");
        println!("  --raw-log, -r      Stream raw events directly to console (stdout)");
        println!("  --help, -h         Show this help message");
        return Ok(());
    }

    let _guard = init_diagnostics(&DiagnosticsConfig::default());
    info!("Starting Trajectory Desktop Capture Agent (Edition 2024)...");

    let print_raw_log = cli_opts.raw_log_console
        || std::env::var("TRAJECTORY_RAW_LOG")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

    let config_path = resolve_config_path(cli_opts.config_path);
    info!("Using configuration: {}", config_path.display());
    let uploader_companion_task = {
        let companion_cfg = config_path.clone();
        tokio::spawn(async move {
            loop {
                if let Some(mut child) = spawn_uploader_companion(&companion_cfg) {
                    let _ = child.wait().await;
                    warn!("Companion uploader process exited. Restarting in 5s...");
                }
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        })
    };
    let runtime_config = ClientRuntimeConfig::from_file(&config_path)?;
    let identity = RuntimeIdentity::from_config(&runtime_config);
    let machine_id = identity.machine_id.as_str();
    let user_id = identity.user_id.as_str();
    let windows_session_id = 1u32;
    let is_running = Arc::new(AtomicBool::new(true));
    let is_running_ctrlc = is_running.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            info!("Ctrl+C signal received. Shutting down capture agent gracefully...");
            is_running_ctrlc.store(false, Ordering::Relaxed);
        }
    });

    // 1. Initialize Event Bus with Priority Shedding
    let event_bus = Arc::new(EventBus::new(EventBusConfig::default()));
    let bus_pub = event_bus.publisher();
    let bus_recv = event_bus.receiver();

    // 2. Initialize Spool & Session Persistence
    let spool_root = runtime_config.spool_dir.clone();
    let global_id_allocator = session::GlobalEventIdAllocator::new(&spool_root)?;
    let global_seq = global_id_allocator.current_atomic();

    let mut session_mgr = SessionManager::start(&spool_root, machine_id, user_id)?;
    let active_session = session_mgr.current_session_id().as_str();
    info!("Active Session: {}", active_session);
    let raw_log_file = spool_root
        .join("recording")
        .join(active_session)
        .join("events.raw.ndjson");
    info!("Raw event log file: {}", raw_log_file.display());
    if print_raw_log {
        info!("Real-time console raw log streaming: ENABLED");
    }

    // 3. Initialize UIA Inspector & Privacy Engine
    let uia_inspector = UiaInspector::new();
    let privacy_engine = PrivacyEngine::new(PrivacyPolicy::default());

    // 4. Initialize Correlation Engine
    let mut correlation_engine = CorrelationEngine::new(
        session_mgr.current_session_id().clone(),
        user_id,
        machine_id,
        global_seq.clone(),
    );

    // 5. Initialize Capture Subsystems (with headless fallback support)
    let input_mgr = InputHookManager::start(machine_id, windows_session_id, user_id)?;
    let window_tracker = WindowTracker::start(machine_id, windows_session_id, user_id)
        .unwrap_or_else(|_| WindowTracker::start_mock(machine_id, windows_session_id, user_id));
    let clipboard_mgr = ClipboardManager::start(machine_id, windows_session_id, user_id)?;
    let file_mgr = FileWatcherManager::start(machine_id, windows_session_id, user_id, vec![])?;

    // 6. Start Named Pipe IPC Server for Browser Extension & Desktop Tray
    let pipe_name = r"\\.\pipe\trajectory-agent-ipc";
    let (ipc_tx, mut ipc_rx) = tokio::sync::mpsc::channel(100);
    let cancel_token = tokio_util::sync::CancellationToken::new();
    let ipc_server = IpcServer::new(pipe_name, ipc_tx, cancel_token.clone());
    info!("Agent IPC Server listening on {}", pipe_name);

    tokio::spawn(async move {
        if let Err(e) = ipc_server.run().await {
            error!("Agent IPC server error: {}", e);
        }
    });

    let pub_for_ipc = bus_pub.clone();
    let global_seq_for_ipc = global_seq.clone();
    tokio::spawn(async move {
        while let Some(msg) = ipc_rx.recv().await {
            match msg {
                IpcMessage::BrowserDomEvent(raw) => {
                    let mut raw = *raw;
                    if raw.global_event_id.is_none_or(|id| id.as_u64() == 0) {
                        raw.global_event_id = Some(core_types::id::GlobalEventId::new(
                            global_seq_for_ipc.fetch_add(1, Ordering::Relaxed),
                        ));
                    }
                    let _ = pub_for_ipc.publish_event(raw);
                }
                _ => {}
            }
        }
    });

    // 7. Route raw input / window / clipboard / file streams into Event Bus
    let input_rx = input_mgr.receiver();
    let win_rx = window_tracker.receiver();
    let clip_rx = clipboard_mgr.receiver();
    let file_rx = file_mgr.receiver();

    let pub_input = bus_pub.clone();
    std::thread::spawn(move || {
        while let Ok(ev) = input_rx.recv() {
            let _ = pub_input.publish_event(ev);
        }
    });

    let pub_win = bus_pub.clone();
    std::thread::spawn(move || {
        while let Ok(ev) = win_rx.recv() {
            let _ = pub_win.publish_event(ev);
        }
    });

    let pub_clip = bus_pub.clone();
    std::thread::spawn(move || {
        while let Ok(ev) = clip_rx.recv() {
            let _ = pub_clip.publish_event(ev);
        }
    });

    let pub_file = bus_pub.clone();
    std::thread::spawn(move || {
        while let Ok(ev) = file_rx.recv() {
            let _ = pub_file.publish_event(ev);
        }
    });

    info!("All capture subsystems and event pipelines online. Processing events...");

    // 8. Main Event Consumption and Correlation Loop
    let mut last_rotation_check = std::time::Instant::now();

    while is_running.load(Ordering::Relaxed) {
        // Drain events from priority event bus with 50ms timeout
        match bus_recv.recv_timeout(Duration::from_millis(50)) {
            Ok((priority, mut raw_event)) => {
                // Query UIA only for semantic events, never for raw mouse movement.
                let target_metadata =
                    target_metadata_for_event(&uia_inspector, &raw_event.payload).await;

                // Correlate into CanonicalAction
                let actions = correlation_engine.process_event(&raw_event, target_metadata);
                for mut action in actions {
                    // Fail-closed privacy redaction
                    privacy_engine.redact_canonical_action(&mut action);
                    // Persist to SQLite WAL database
                    let _ = session_mgr.write_canonical_action(&action);
                }

                // Raw events are only persisted after correlation has consumed
                // the in-memory event. This prevents disk artifacts from
                // becoming a reconstructable keyboard log.
                privacy_engine.redact_raw_event(&mut raw_event);
                let _ = session_mgr.write_raw_event(&raw_event);
                if print_raw_log {
                    if let Ok(json_str) = serde_json::to_string(&raw_event) {
                        println!("[RAW LOG] {json_str}");
                    }
                }
            }
            Err(_) => {
                // Timeout: periodic flush of typing/scroll burst aggregators
                let flushed_actions = correlation_engine.periodic_flush();
                for mut action in flushed_actions {
                    privacy_engine.redact_canonical_action(&mut action);
                    let _ = session_mgr.write_canonical_action(&action);
                }
            }
        }

        // Check for hourly session boundary rotation
        if last_rotation_check.elapsed() >= Duration::from_secs(10) {
            last_rotation_check = std::time::Instant::now();
            if let Ok(Some(rotated_old_session)) = session_mgr.check_rotation() {
                info!(
                    "Hourly boundary reached: rotated session {}",
                    rotated_old_session.as_str()
                );
                correlation_engine.set_session_id(session_mgr.current_session_id().clone());
            }
        }
    }

    if let Ok(id) = session_mgr.finalize_active_session() {
        info!("Active session {} finalized for upload", id.as_str());
    } else {
        let _ = session_mgr.flush();
    }
    uploader_companion_task.abort();
    info!("Flushing pending uploads via companion uploader...");
    tokio::time::sleep(Duration::from_secs(3)).await;
    info!("Trajectory Desktop Capture Agent stopped cleanly.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::ClientRuntimeConfig;
    use core_types::event::{RawEventPayload, RawKeyboardEvent, RawMouseEvent};

    #[test]
    fn uia_lookup_ignores_mouse_moves_but_keeps_semantic_targets() {
        let mouse_move = RawEventPayload::Mouse(RawMouseEvent {
            event_type: "MOUSE_MOVE".to_string(),
            physical_x: 10,
            physical_y: 20,
            ..Default::default()
        });
        assert_eq!(uia_lookup_request(&mouse_move), None);

        let mouse_up = RawEventPayload::Mouse(RawMouseEvent {
            event_type: "MOUSE_UP".to_string(),
            physical_x: 10,
            physical_y: 20,
            ..Default::default()
        });
        assert_eq!(
            uia_lookup_request(&mouse_up),
            Some(UiaLookupRequest::Point(10, 20))
        );

        let key_down = RawEventPayload::Keyboard(RawKeyboardEvent {
            event_type: "KEY_DOWN".to_string(),
            ..Default::default()
        });
        assert_eq!(
            uia_lookup_request(&key_down),
            Some(UiaLookupRequest::Focused)
        );

        let key_up = RawEventPayload::Keyboard(RawKeyboardEvent {
            event_type: "KEY_UP".to_string(),
            ..Default::default()
        });
        assert_eq!(uia_lookup_request(&key_up), None);
    }

    #[test]
    fn runtime_identity_requires_enrolled_machine_and_user_ids() {
        assert!(RuntimeIdentity::from_values(None, Some("operator-01".to_string())).is_err());
        assert!(RuntimeIdentity::from_values(Some("MACHINE-01".to_string()), None).is_err());

        let identity = RuntimeIdentity::from_values(
            Some("MACHINE-01".to_string()),
            Some("operator-01".to_string()),
        )
        .expect("enrollment identity should be accepted");
        assert_eq!(identity.machine_id, "MACHINE-01");
        assert_eq!(identity.user_id, "operator-01");
    }

    #[test]
    fn capture_runtime_rejects_non_client_role_and_uses_configured_spool() {
        let rejected = ClientRuntimeConfig::from_pairs([
            ("DEPLOYMENT_ROLE", "server"),
            ("TRAJECTORY_SERVER_URL", "https://collector.example.test"),
            ("TRAJECTORY_MACHINE_ID", "MACHINE-01"),
            ("TRAJECTORY_USER_ID", "operator-01"),
            ("SPOOL_DIR", r"C:\\ProgramData\\TrajectoryRecorder\\spool"),
        ]);
        assert!(rejected.is_err());

        let accepted = ClientRuntimeConfig::from_pairs([
            ("DEPLOYMENT_ROLE", "client"),
            ("TRAJECTORY_SERVER_URL", "https://collector.example.test"),
            ("TRAJECTORY_MACHINE_ID", "MACHINE-01"),
            ("TRAJECTORY_USER_ID", "operator-01"),
            ("SPOOL_DIR", r"C:\\ProgramData\\TrajectoryRecorder\\spool"),
        ])
        .expect("client configuration should load");
        assert_eq!(
            accepted.spool_dir,
            PathBuf::from(r"C:\\ProgramData\\TrajectoryRecorder\\spool")
        );

        let relative_spool = ClientRuntimeConfig::from_pairs([
            ("DEPLOYMENT_ROLE", "client"),
            ("TRAJECTORY_SERVER_URL", "https://collector.example.test"),
            ("TRAJECTORY_MACHINE_ID", "MACHINE-01"),
            ("TRAJECTORY_USER_ID", "operator-01"),
            ("SPOOL_DIR", "spool"),
        ]);
        assert!(relative_spool.is_err());
    }

    #[test]
    fn cli_options_parsing_supports_raw_log_and_config() {
        let args = vec![
            "trajectory-agent".to_string(),
            "--raw-log".to_string(),
            "--config".to_string(),
            r"C:\Custom\client.env".to_string(),
        ];
        let opts = parse_cli_options(&args).expect("should parse CLI options");
        assert!(opts.raw_log_console);
        assert_eq!(opts.config_path, Some(PathBuf::from(r"C:\Custom\client.env")));

        let path = capture_config_path(&args).expect("capture_config_path");
        assert_eq!(path, PathBuf::from(r"C:\Custom\client.env"));
    }

    #[test]
    fn cli_options_parsing_supports_short_flag_r() {
        let args = vec!["trajectory-agent".to_string(), "-r".to_string()];
        let opts = parse_cli_options(&args).expect("should parse -r");
        assert!(opts.raw_log_console);
    }
}
