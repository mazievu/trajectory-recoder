//! Trajectory Desktop Capture Agent binary.
//! Integrates Win32 hooks, window tracking, clipboard, file events, UIA, privacy filtering,
//! correlation engine, NDJSON & SQLite WAL persistence, and Named Pipe IPC server.

use clipboard_win::ClipboardManager;
use correlator::CorrelationEngine;
use core_types::event::RawEventPayload;
use core_types::metadata::TargetMetadata;
use diagnostics::{init_diagnostics, DiagnosticsConfig};
use event_bus::bus::{EventBus, EventBusConfig};
use file_events_win::FileWatcherManager;
use input_win::manager::InputHookManager;
use ipc::{IpcMessage, IpcServer};
use privacy::engine::{PrivacyEngine, PrivacyPolicy};
use session::manager::SessionManager;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};
use uia_win::inspector::UiaInspector;
use window_win::tracker::WindowTracker;

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
            ) => Some(UiaLookupRequest::Point(
            mouse.physical_x,
            mouse.physical_y,
        )),
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
    let _guard = init_diagnostics(&DiagnosticsConfig::default());
    info!("Starting Trajectory Desktop Capture Agent (Edition 2024)...");

    let machine_id = "WORKSTATION-01";
    let user_id = "user_primary";
    let windows_session_id = 1u32;
    let is_running = Arc::new(AtomicBool::new(true));

    // 1. Initialize Event Bus with Priority Shedding
    let event_bus = Arc::new(EventBus::new(EventBusConfig::default()));
    let bus_pub = event_bus.publisher();
    let bus_recv = event_bus.receiver();

    // 2. Initialize Spool & Session Persistence
    let spool_root = PathBuf::from("spool");
    let global_id_allocator = session::GlobalEventIdAllocator::new(&spool_root)?;
    let global_seq = global_id_allocator.current_atomic();

    let mut session_mgr = SessionManager::start(&spool_root, machine_id, user_id)?;
    info!("Active Session: {}", session_mgr.current_session_id().as_str());

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
    tokio::spawn(async move {
        while let Some(msg) = ipc_rx.recv().await {
            match msg {
                IpcMessage::BrowserDomEvent(raw) => {
                    let _ = pub_for_ipc.publish_event(*raw);
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
                // Privacy redaction on raw event
                privacy_engine.redact_raw_event(&mut raw_event);

                // Write raw event to NDJSON log
                let _ = session_mgr.write_raw_event(&raw_event);

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
                info!("Hourly boundary reached: rotated session {}", rotated_old_session.as_str());
                correlation_engine.set_session_id(session_mgr.current_session_id().clone());
            }
        }
    }

    info!("Trajectory Desktop Capture Agent stopped cleanly.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
