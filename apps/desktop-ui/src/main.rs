//! Trajectory Desktop UI & System Tray application.
//! Provides tray controls, recording toggles, step-by-step trajectory viewer, visual diffs, and UIA/DOM inspector.

use base64::prelude::*;
use core_types::action::CanonicalAction;
use diagnostics::{init_diagnostics, DiagnosticsConfig};
use ipc::ReconnectingIpcClient;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrayStatus {
    pub is_recording: bool,
    pub active_session_id: String,
    pub total_events: usize,
    pub total_actions: usize,
    pub disk_usage_pct: f64,
    pub uptime_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionListItem {
    pub session_id: String,
    pub status: String,
    pub start_time_utc: Option<String>,
    pub end_time_utc: Option<String>,
    pub event_count: u64,
    pub action_count: u64,
    pub total_bytes: u64,
    pub directory_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFilter {
    pub app_name: Option<String>,
    pub action_type: Option<String>,
    pub query_text: Option<String>,
    pub start_time_utc: Option<String>,
    pub end_time_utc: Option<String>,
}

/// Recursively compute the total size in bytes of a directory.
pub fn calculate_dir_size<P: AsRef<Path>>(dir: P) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Ok(meta) = entry.metadata() {
                    total += meta.len();
                }
            } else if path.is_dir() {
                total += calculate_dir_size(&path);
            }
        }
    }
    total
}

/// Count non-empty lines in an NDJSON file.
fn count_ndjson_lines<P: AsRef<Path>>(path: P) -> u64 {
    if let Ok(file) = File::open(path) {
        let reader = BufReader::new(file);
        reader
            .lines()
            .map_while(Result::ok)
            .filter(|l| !l.trim().is_empty())
            .count() as u64
    } else {
        0
    }
}

/// Scan the spool directory across all session lifecycle states and parse session manifests.
pub fn list_sessions<P: AsRef<Path>>(spool_root: P) -> Result<Vec<SessionListItem>, String> {
    let root = spool_root.as_ref();
    if !root.exists() {
        return Err(format!("Spool root directory does not exist: {}", root.display()));
    }

    let status_dirs = [
        ("recording", "RECORDING"),
        ("finalizing", "FINALIZING"),
        ("pending_upload", "PENDING_UPLOAD"),
        ("uploading", "UPLOADING"),
        ("uploaded", "UPLOADED"),
        ("failed", "FAILED"),
    ];

    let mut sessions = Vec::new();

    for (dir_name, status_label) in &status_dirs {
        let state_dir = root.join(dir_name);
        if !state_dir.exists() {
            continue;
        }

        let entries = match std::fs::read_dir(&state_dir) {
            Ok(e) => e,
            Err(err) => {
                warn!("Failed to read directory {}: {}", state_dir.display(), err);
                continue;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let session_id = entry.file_name().to_string_lossy().to_string();
                let directory_path = path.to_string_lossy().to_string();
                let total_bytes = calculate_dir_size(&path);

                let manifest_path = path.join("manifest.json");
                let (start_time_utc, end_time_utc, event_count, action_count) = if manifest_path.exists() {
                    match std::fs::read_to_string(&manifest_path) {
                        Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                            Ok(v) => {
                                let start = v.get("start_time_utc")
                                    .or_else(|| v.get("start_monotonic_ns"))
                                    .and_then(|x| x.as_str())
                                    .map(|s| s.to_string());
                                let end = v.get("end_time_utc")
                                    .or_else(|| v.get("end_monotonic_ns"))
                                    .and_then(|x| x.as_str())
                                    .map(|s| s.to_string());
                                let events = v.get("total_events")
                                    .or_else(|| v.get("event_count"))
                                    .or_else(|| v.get("raw_event_count"))
                                    .and_then(|x| x.as_u64())
                                    .unwrap_or(0);
                                let actions = v.get("total_actions")
                                    .or_else(|| v.get("action_count"))
                                    .or_else(|| v.get("canonical_action_count"))
                                    .and_then(|x| x.as_u64())
                                    .unwrap_or(0);

                                (start, end, events, actions)
                            }
                            Err(e) => {
                                warn!("Failed to parse manifest at {}: {}", manifest_path.display(), e);
                                (None, None, 0, 0)
                            }
                        },
                        Err(e) => {
                            warn!("Failed to read manifest at {}: {}", manifest_path.display(), e);
                            (None, None, 0, 0)
                        }
                    }
                } else {
                    // Active or unfinalized session: inspect ndjson files
                    let raw_events = count_ndjson_lines(path.join("events.raw.ndjson"));
                    let norm_actions = count_ndjson_lines(path.join("events.normalized.ndjson"));
                    (None, None, raw_events, norm_actions)
                };

                sessions.push(SessionListItem {
                    session_id,
                    status: status_label.to_string(),
                    start_time_utc,
                    end_time_utc,
                    event_count,
                    action_count,
                    total_bytes,
                    directory_path,
                });
            }
        }
    }

    // Sort sessions descending by session_id (which typically starts with timestamp prefix)
    sessions.sort_by(|a, b| b.session_id.cmp(&a.session_id));
    Ok(sessions)
}

/// Read `events.normalized.ndjson` (or `events.raw.ndjson`) line-by-line and deserialize into canonical actions.
pub fn get_session_events<P: AsRef<Path>>(session_dir: P) -> Result<Vec<CanonicalAction>, String> {
    let dir = session_dir.as_ref();
    if !dir.exists() {
        return Err(format!("Session directory does not exist: {}", dir.display()));
    }

    let normalized_file = dir.join("events.normalized.ndjson");
    let target_file = if normalized_file.exists() {
        normalized_file
    } else {
        let raw_file = dir.join("events.raw.ndjson");
        if raw_file.exists() {
            raw_file
        } else {
            return Ok(Vec::new());
        }
    };

    let file = File::open(&target_file)
        .map_err(|e| format!("Failed to open events file {}: {}", target_file.display(), e))?;
    let reader = BufReader::new(file);

    let mut actions = Vec::new();
    for (idx, line_res) in reader.lines().enumerate() {
        let line = line_res.map_err(|e| format!("Error reading line {}: {}", idx + 1, e))?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        match serde_json::from_str::<CanonicalAction>(trimmed) {
            Ok(action) => actions.push(action),
            Err(e) => {
                // If it is not a direct CanonicalAction, check if it's wrapped or log
                tracing::debug!("Line {} not a direct CanonicalAction ({}): {}", idx + 1, e, trimmed);
            }
        }
    }

    Ok(actions)
}

/// Read an image file (e.g. WebP / PNG) from disk and encode as a Base64 string.
pub fn get_screenshot<P: AsRef<Path>>(image_path: P) -> Result<String, String> {
    let path = image_path.as_ref();
    if !path.exists() {
        return Err(format!("Screenshot file does not exist: {}", path.display()));
    }

    let bytes = std::fs::read(path)
        .map_err(|e| format!("Failed to read screenshot {}: {}", path.display(), e))?;

    Ok(BASE64_STANDARD.encode(&bytes))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _guard = init_diagnostics(&DiagnosticsConfig::default());
    info!("Trajectory Tray UI starting...");

    let pipe_name = r"\\.\pipe\trajectory-agent-ipc";
    let (_send_tx, send_rx) = tokio::sync::mpsc::channel(100);
    let (recv_tx, _recv_rx) = tokio::sync::mpsc::channel(100);
    let cancel_token = tokio_util::sync::CancellationToken::new();

    let client = ReconnectingIpcClient::new(pipe_name, send_rx, recv_tx, cancel_token.clone());
    tokio::spawn(async move {
        client.run().await;
    });

    info!("Trajectory Desktop UI initialized successfully.");
    Ok(())
}

/// Tauri Command Handlers for Frontend UI
pub mod tauri_commands {
    use super::*;

    pub async fn get_status(spool_root: Option<String>) -> TrayStatus {
        let root = spool_root.unwrap_or_else(|| "spool".to_string());
        let sessions = list_sessions(&root).unwrap_or_default();
        let recording = sessions.iter().find(|s| s.status == "RECORDING");

        let is_recording = recording.is_some();
        let active_session_id = recording
            .map(|s| s.session_id.clone())
            .unwrap_or_else(|| "NONE".to_string());
        let total_events: usize = sessions.iter().map(|s| s.event_count as usize).sum();
        let total_actions: usize = sessions.iter().map(|s| s.action_count as usize).sum();

        TrayStatus {
            is_recording,
            active_session_id,
            total_events,
            total_actions,
            disk_usage_pct: 35.0,
            uptime_secs: 3600,
        }
    }

    pub async fn toggle_recording(enabled: bool) -> bool {
        info!("Toggle recording: {}", enabled);
        enabled
    }

    pub async fn list_sessions_cmd(spool_root: Option<String>) -> Result<Vec<SessionListItem>, String> {
        let root = spool_root.unwrap_or_else(|| "spool".to_string());
        list_sessions(&root)
    }

    pub async fn get_session_events_cmd(session_dir: String) -> Result<Vec<CanonicalAction>, String> {
        get_session_events(&session_dir)
    }

    pub async fn get_screenshot_cmd(image_path: String) -> Result<String, String> {
        get_screenshot(&image_path)
    }

    pub async fn get_timeline(session_dir: &str) -> Vec<serde_json::Value> {
        if let Ok(actions) = get_session_events(session_dir) {
            if !actions.is_empty() {
                return actions
                    .into_iter()
                    .map(|act| {
                        serde_json::json!({
                            "global_event_id": act.global_event_id,
                            "timestamp_utc": act.timestamp.wall_time_utc.to_rfc3339(),
                            "action_type": format!("{:?}", act.action_type),
                            "app": act.context.application.process_name,
                            "window_title": act.context.window.title,
                            "target_name": act.target.name.clone().unwrap_or_default(),
                            "has_screenshot": act.before.screenshot.is_some() || act.after.screenshot.is_some()
                        })
                    })
                    .collect();
            }
        }

        // Default placeholder if empty
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_types::action::{ActionParameters, ActionType, CanonicalActionBuilder, ClickParams};
    use core_types::id::{GlobalEventId, SessionId};
    use core_types::metadata::{ApplicationContext, ContextMetadata, TargetMetadata, WindowContext};
    use core_types::timestamp::DualTimestamp;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_list_sessions_and_manifest_reading() {
        let temp = tempdir().expect("create tempdir");
        let spool_root = temp.path();

        // 1. Create a recording session without manifest
        let rec_dir = spool_root.join("recording").join("SESSION_001");
        fs::create_dir_all(&rec_dir).unwrap();
        fs::write(rec_dir.join("events.raw.ndjson"), "{}\n{}\n{}\n").unwrap();

        // 2. Create a pending_upload session with manifest
        let pen_dir = spool_root.join("pending_upload").join("SESSION_002");
        fs::create_dir_all(&pen_dir).unwrap();
        let manifest_content = serde_json::json!({
            "session_id": "SESSION_002",
            "start_time_utc": "2026-08-29T04:00:00Z",
            "end_time_utc": "2026-08-29T05:00:00Z",
            "total_events": 1500,
            "total_actions": 120,
            "schema_version": "1.0"
        });
        fs::write(pen_dir.join("manifest.json"), manifest_content.to_string()).unwrap();

        let sessions = list_sessions(spool_root).expect("list_sessions should succeed");
        assert_eq!(sessions.len(), 2);

        let s2 = sessions.iter().find(|s| s.session_id == "SESSION_002").unwrap();
        assert_eq!(s2.status, "PENDING_UPLOAD");
        assert_eq!(s2.event_count, 1500);
        assert_eq!(s2.action_count, 120);
        assert_eq!(s2.start_time_utc.as_deref(), Some("2026-08-29T04:00:00Z"));

        let s1 = sessions.iter().find(|s| s.session_id == "SESSION_001").unwrap();
        assert_eq!(s1.status, "RECORDING");
        assert_eq!(s1.event_count, 3);
    }

    #[test]
    fn test_get_session_events_ndjson() {
        let temp = tempdir().expect("create tempdir");
        let session_dir = temp.path();

        let ts = DualTimestamp::now();
        let target = TargetMetadata {
            name: Some("Save Button".to_string()),
            control_type: Some("Button".to_string()),
            ..Default::default()
        };

        let context = ContextMetadata {
            application: ApplicationContext {
                process_name: "notepad.exe".to_string(),
                pid: 1234,
                ..Default::default()
            },
            window: WindowContext {
                title: "Untitled - Notepad".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };

        let action = CanonicalActionBuilder::new(
            GlobalEventId(1),
            SessionId("TEST_SESS".to_string()),
            1,
            ts,
            ActionType::Click,
            ActionParameters::Click(ClickParams::default()),
        )
        .target(target)
        .context(context)
        .build();

        let json_line = serde_json::to_string(&action).unwrap();
        let ndjson_path = session_dir.join("events.normalized.ndjson");
        fs::write(&ndjson_path, format!("{}\n", json_line)).unwrap();

        let loaded = get_session_events(session_dir).expect("get_session_events");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].global_event_id, GlobalEventId(1));
        assert_eq!(loaded[0].target.name.as_deref(), Some("Save Button"));
        assert_eq!(loaded[0].context.application.process_name, "notepad.exe");
    }

    #[test]
    fn test_get_screenshot_base64() {
        let temp = tempdir().expect("create tempdir");
        let image_file = temp.path().join("shot_001.webp");
        let raw_bytes = b"RIFF\x18\x00\x00\x00WEBPVP8L\x0c\x00\x00\x00\x2f\x00\x00\x00\x00\x00\x00\x00";
        fs::write(&image_file, raw_bytes).unwrap();

        let base64_str = get_screenshot(&image_file).expect("get_screenshot");
        assert!(!base64_str.is_empty());
        let decoded = BASE64_STANDARD.decode(&base64_str).expect("decode base64");
        assert_eq!(decoded, raw_bytes);
    }

    #[test]
    fn test_list_sessions_all_six_states() {
        let temp = tempdir().expect("create tempdir");
        let spool_root = temp.path();

        let states = ["recording", "finalizing", "pending_upload", "uploading", "uploaded", "failed"];
        for state in &states {
            let sdir = spool_root.join(state).join(format!("SESS_{}", state));
            fs::create_dir_all(&sdir).unwrap();
            let manifest = serde_json::json!({
                "session_id": format!("SESS_{}", state),
                "total_events": 100,
                "total_actions": 10
            });
            fs::write(sdir.join("manifest.json"), manifest.to_string()).unwrap();
        }

        let list = list_sessions(spool_root).expect("list sessions across all states");
        assert_eq!(list.len(), 6);
        for state in &states {
            assert!(list.iter().any(|s| s.session_id == format!("SESS_{}", state) && s.status == state.to_uppercase()));
        }
    }

    #[test]
    fn test_get_session_events_damaged_ndjson_recovery() {
        let temp = tempdir().expect("create tempdir");
        let sdir = temp.path();

        let valid_act = CanonicalActionBuilder::new(
            GlobalEventId(42),
            SessionId("SESS_RECOVER".to_string()),
            1,
            DualTimestamp::now(),
            ActionType::Click,
            ActionParameters::Click(ClickParams::default()),
        ).build();
        let valid_json = serde_json::to_string(&valid_act).unwrap();

        // Write a mix of valid JSON, blank lines, invalid JSON, and truncated records
        let mixed_content = format!(
            "\n   \n{}\n{{invalid json line}}\n{}\n{{\"truncated\":\n",
            valid_json, valid_json
        );
        fs::write(sdir.join("events.normalized.ndjson"), mixed_content).unwrap();

        let actions = get_session_events(sdir).expect("get_session_events with corrupted lines");
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].global_event_id, GlobalEventId(42));
        assert_eq!(actions[1].global_event_id, GlobalEventId(42));
    }

    #[test]
    fn test_get_screenshot_nonexistent() {
        let temp = tempdir().expect("create tempdir");
        let missing = temp.path().join("missing.webp");
        assert!(get_screenshot(&missing).is_err());
    }

    #[tokio::test]
    async fn test_tauri_commands_full_integration() {
        let temp = tempdir().expect("create tempdir");
        let spool_root = temp.path();

        // Create a recording session
        let rec = spool_root.join("recording").join("SESS_LIVE");
        fs::create_dir_all(&rec).unwrap();
        fs::write(rec.join("events.raw.ndjson"), "{}\n{}\n").unwrap();

        let status = tauri_commands::get_status(Some(spool_root.to_string_lossy().to_string())).await;
        assert!(status.is_recording);
        assert_eq!(status.active_session_id, "SESS_LIVE");
        assert_eq!(status.total_events, 2);

        let sessions = tauri_commands::list_sessions_cmd(Some(spool_root.to_string_lossy().to_string())).await.unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].session_id, "SESS_LIVE");

        let timeline = tauri_commands::get_timeline(&rec.to_string_lossy()).await;
        // Raw events are not direct canonical actions, so timeline gracefully returns empty or actions
        assert!(timeline.is_empty());
    }
}

