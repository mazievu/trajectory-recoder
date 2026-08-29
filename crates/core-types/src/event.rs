use crate::id::GlobalEventId;
use crate::metadata::{BoundingRect, ModifierState, MouseButton, Point2D};
use crate::timestamp::DualTimestamp;
use crate::{SCHEMA_IDENTIFIER, SCHEMA_VERSION};
use serde::{Deserialize, Serialize};

/// Source module that generated the raw event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventSource {
    #[default]
    Win32Hook,
    InputHook,
    WinEvent,
    UiAutomation,
    BrowserExtension,
    ClipboardListener,
    FileWatcher,
    WgcScreenCapture,
    SystemTelemetry,
    SessionRouter,
}

/// Uncorrelated raw event record persisted post-privacy filtering.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawEvent {
    pub schema: String,
    pub schema_version: String,
    pub event_id: u64,
    #[serde(default)]
    pub global_event_id: Option<GlobalEventId>,
    pub timestamp: DualTimestamp,
    pub machine_id: String,
    pub windows_session_id: u32,
    pub user_id: String,
    pub source: EventSource,
    pub source_sequence: u64,
    pub payload: RawEventPayload,
}

impl RawEvent {
    pub fn new(
        event_id: u64,
        global_event_id: GlobalEventId,
        timestamp: DualTimestamp,
        machine_id: String,
        windows_session_id: u32,
        user_id: String,
        source: EventSource,
        source_sequence: u64,
        payload: RawEventPayload,
    ) -> Self {
        Self {
            schema: SCHEMA_IDENTIFIER.to_string(),
            schema_version: SCHEMA_VERSION.to_string(),
            event_id,
            global_event_id: Some(global_event_id),
            timestamp,
            machine_id,
            windows_session_id,
            user_id,
            source,
            source_sequence,
            payload,
        }
    }
}

/// Payload variants for all raw events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum RawEventPayload {
    Mouse(RawMouseEvent),
    Keyboard(RawKeyboardEvent),
    Window(RawWindowEvent),
    UiAutomation(RawUiaEvent),
    Browser(RawBrowserEvent),
    Clipboard(RawClipboardEvent),
    File(RawFileEvent),
    Screen(RawScreenEvent),
    System(RawSystemEvent),
    Session(RawSessionEvent),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RawMouseEvent {
    pub event_type: String, // "MOUSE_DOWN", "MOUSE_UP", "MOUSE_MOVE", "MOUSE_WHEEL"
    pub button: MouseButton,
    pub coords: Point2D,
    pub monitor_id: u32,
    pub delta_x: f64,
    pub delta_y: f64,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub physical_x: i32,
    #[serde(default)]
    pub physical_y: i32,
    #[serde(default)]
    pub normalized_x: f32,
    #[serde(default)]
    pub normalized_y: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RawKeyboardEvent {
    pub event_type: String, // "KEY_DOWN", "KEY_UP"
    pub vk_code: u32,
    pub scan_code: u32,
    pub key_name: String,
    pub modifiers: ModifierState,
    pub is_injected: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RawWindowEvent {
    pub event_type: String, // "FOREGROUND", "MOVE", "RESIZE", "MINIMIZE", "MAXIMIZE", "CLOSE"
    pub hwnd: u64,
    pub pid: u32,
    pub process_name: String,
    pub window_title: String,
    pub bounds: BoundingRect,
    pub monitor_id: u32,
    pub dpi: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RawUiaEvent {
    pub event_type: String,
    pub control_type: String,
    pub name: Option<String>,
    pub automation_id: Option<String>,
    pub class_name: Option<String>,
    pub framework_id: Option<String>,
    pub bounds: BoundingRect,
    pub is_password: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RawBrowserEvent {
    pub tab_id: u32,
    pub event_type: String,
    pub url: String,
    pub tag_name: String,
    pub target_id: Option<String>,
    pub target_class: Option<String>,
    pub target_text: Option<String>,
    pub css_selector: Option<String>,
    pub xpath: Option<String>,
    pub bounds: BoundingRect,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RawClipboardEvent {
    pub format: String,
    pub byte_length: usize,
    pub hash_sha256: String,
    pub source_hwnd: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RawFileEvent {
    pub action: String, // "CREATED", "MODIFIED", "DELETED", "RENAMED"
    pub file_path: String,
    pub old_file_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RawScreenEvent {
    pub event_type: String, // "SCREENSHOT_CAPTURED", "TOPOLOGY_CHANGED"
    pub monitor_id: u32,
    pub screenshot_file: Option<String>,
    pub diff_ratio: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RawSystemEvent {
    pub event_type: String, // "LOCK", "UNLOCK", "SLEEP", "RESUME", "LOGON", "LOGOFF"
    pub details: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RawSessionEvent {
    pub event_type: String, // "SESSION_START", "SESSION_ROTATE", "SESSION_FINALIZE"
    pub session_id: String,
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raw_event_all_payload_variants_serde() {
        let ts = DualTimestamp::from_parts(chrono::Utc::now(), 100, 0);

        let payloads = vec![
            RawEventPayload::Mouse(RawMouseEvent::default()),
            RawEventPayload::Keyboard(RawKeyboardEvent::default()),
            RawEventPayload::Window(RawWindowEvent::default()),
            RawEventPayload::UiAutomation(RawUiaEvent::default()),
            RawEventPayload::Browser(RawBrowserEvent::default()),
            RawEventPayload::Clipboard(RawClipboardEvent::default()),
            RawEventPayload::File(RawFileEvent::default()),
            RawEventPayload::Screen(RawScreenEvent::default()),
            RawEventPayload::System(RawSystemEvent::default()),
            RawEventPayload::Session(RawSessionEvent::default()),
        ];

        for (i, p) in payloads.into_iter().enumerate() {
            let event = RawEvent::new(
                (i + 1) as u64,
                GlobalEventId::new((i + 1) as u64),
                ts,
                "machine_1".to_string(),
                1,
                "user_1".to_string(),
                EventSource::Win32Hook,
                (i + 1) as u64,
                p,
            );

            let json = serde_json::to_string(&event).unwrap();
            let deserialized: RawEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(event, deserialized);
        }
    }
}
