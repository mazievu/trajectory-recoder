use crate::id::{GlobalEventId, SessionId};
use crate::metadata::{ContextMetadata, ModifierState, MouseButton, Point2D, ScrollDirection, TargetMetadata};
use crate::state::{ActionEvidence, StateEvidence, StateSnapshot};
use crate::timestamp::DualTimestamp;
use crate::{SCHEMA_IDENTIFIER, SCHEMA_VERSION};
use serde::{Deserialize, Serialize};

/// Comprehensive taxonomy of all 39 canonical action types supported by the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ActionType {
    // Mouse Interaction
    Click,
    DoubleClick,
    RightClick,
    MiddleClick,
    DragDrop,
    Scroll,

    // Keyboard Interaction
    TypeText,
    KeyPress,
    Shortcut,

    // Clipboard
    Copy,
    Cut,
    Paste,

    // Window Lifecycle
    WindowSwitch,
    WindowOpen,
    WindowClose,

    // Application Lifecycle
    AppOpen,
    AppClose,

    // Web Browser Navigation
    Navigate,

    // User Workflow File Operations
    FileOpen,
    FileSave,
    FileSaveAs,
    FileCreate,
    FileCopy,
    FileMove,
    FileRename,
    FileDelete,
    FileUpload,
    FileDownload,
    FileExport,

    // Dialogs
    DialogOpen,
    DialogConfirm,
    DialogCancel,

    // System & Workstation State
    Wait,
    UserIdle,
    SystemLock,
    SystemUnlock,
    SystemSleep,
    SystemResume,

    // Fallback
    UnknownInteraction,
}

/// Persisted canonical user interaction with before/after state snapshots and evidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanonicalAction {
    pub schema: String,
    pub schema_version: String,
    pub global_event_id: GlobalEventId,
    pub session_id: SessionId,
    pub session_event_id: u64,
    pub timestamp: DualTimestamp,
    pub action_type: ActionType,
    pub confidence: f32,
    pub target: TargetMetadata,
    pub context: ContextMetadata,
    #[serde(default)]
    pub before: StateSnapshot,
    #[serde(default)]
    pub parameters: ActionParameters,
    #[serde(default)]
    pub after: StateSnapshot,
    #[serde(default)]
    pub evidence: ActionEvidence,
    #[serde(default)]
    pub state_evidence: Option<StateEvidence>,
    pub duration_ms: Option<u64>,
}

pub type ActionDetail = ActionParameters;

/// Specific parameters for each action type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum ActionParameters {
    #[default]
    None,
    Click(ClickParams),
    DoubleClick(ClickParams),
    RightClick(ClickParams),
    MiddleClick(ClickParams),
    TypeText(TypeTextParams),
    KeyPress(KeyPressParams),
    Shortcut(ShortcutParams),
    Scroll(ScrollParams),
    DragDrop(DragDropParams),
    Clipboard(ClipboardParams),
    Window(WindowLifecycleParams),
    File(FileOperationParams),
    Dialog(DialogParams),
    Wait(WaitParams),
    Navigation(NavigationParams),
    System(SystemStateParams),
    Unknown(UnknownParams),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ClickParams {
    pub button: MouseButton,
    pub click_count: u32,
    pub physical_coords: Point2D,
    pub normalized_coords: Point2D,
    pub monitor_id: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TypeTextParams {
    pub text: String,
    pub length: usize,
    pub is_redacted: bool,
    pub character_count: usize,
    pub backspace_count: usize,
    pub enter_pressed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct KeyPressParams {
    pub key_code: u32,
    pub key_name: String,
    pub modifiers: ModifierState,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ShortcutParams {
    pub combination: String,
    pub modifiers: ModifierState,
    pub primary_key: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScrollParams {
    pub delta_x: f64,
    pub delta_y: f64,
    pub direction: ScrollDirection,
    pub container_type: Option<String>,
    pub wheel_ticks: u32,
}

impl Default for ScrollParams {
    fn default() -> Self {
        Self {
            delta_x: 0.0,
            delta_y: 0.0,
            direction: ScrollDirection::VerticalDown,
            container_type: None,
            wheel_ticks: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DragDropParams {
    pub start_coords: Point2D,
    pub end_coords: Point2D,
    pub distance_px: f64,
    pub path_summary: Option<String>,
    pub source_target: Option<String>,
    pub destination_target: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ClipboardParams {
    pub operation: String, // "COPY", "CUT", "PASTE"
    pub content_type: String,
    pub byte_length: usize,
    pub hash_sha256: String,
    pub source_app: Option<String>,
    pub destination_app: Option<String>,
    pub redacted_preview: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct WindowLifecycleParams {
    pub hwnd: u64,
    pub event_type: String,
    pub process_name: String,
    pub window_title: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct FileOperationParams {
    pub operation: String, // "OPEN", "SAVE", "UPLOAD", etc.
    pub file_path: String,
    pub file_name: String,
    pub extension: String,
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DialogParams {
    pub dialog_type: String, // "OPEN", "SAVE_AS", "CONFIRM", "CANCEL"
    pub title: String,
    pub selected_path: Option<String>,
    pub selected_filter: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct WaitParams {
    pub duration_secs: f64,
    pub trigger_reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NavigationParams {
    pub url: String,
    pub transition_type: String,
    pub is_spa_transition: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SystemStateParams {
    pub state_event: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct UnknownParams {
    pub raw_summary: String,
}

/// Fluent builder for constructing verified CanonicalAction instances.
pub struct CanonicalActionBuilder {
    global_event_id: GlobalEventId,
    session_id: SessionId,
    session_event_id: u64,
    timestamp: DualTimestamp,
    action_type: ActionType,
    confidence: f32,
    target: TargetMetadata,
    context: ContextMetadata,
    before: StateSnapshot,
    parameters: ActionParameters,
    after: StateSnapshot,
    evidence: ActionEvidence,
    state_evidence: Option<StateEvidence>,
    duration_ms: Option<u64>,
}

impl CanonicalActionBuilder {
    pub fn new(
        global_event_id: GlobalEventId,
        session_id: SessionId,
        session_event_id: u64,
        timestamp: DualTimestamp,
        action_type: ActionType,
        parameters: ActionParameters,
    ) -> Self {
        Self {
            global_event_id,
            session_id,
            session_event_id,
            timestamp,
            action_type,
            confidence: 1.0,
            target: TargetMetadata::default(),
            context: ContextMetadata::default(),
            before: StateSnapshot::default(),
            parameters,
            after: StateSnapshot::default(),
            evidence: ActionEvidence::default(),
            state_evidence: None,
            duration_ms: None,
        }
    }

    pub fn confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    pub fn target(mut self, target: TargetMetadata) -> Self {
        self.target = target;
        self
    }

    pub fn context(mut self, context: ContextMetadata) -> Self {
        self.context = context;
        self
    }

    pub fn before(mut self, before: StateSnapshot) -> Self {
        self.before = before;
        self
    }

    pub fn after(mut self, after: StateSnapshot) -> Self {
        self.after = after;
        self
    }

    pub fn evidence(mut self, evidence: ActionEvidence) -> Self {
        self.evidence = evidence;
        self
    }

    pub fn state_evidence(mut self, state_evidence: StateEvidence) -> Self {
        self.state_evidence = Some(state_evidence);
        self
    }

    pub fn duration_ms(mut self, duration_ms: u64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }

    pub fn build(self) -> CanonicalAction {
        CanonicalAction {
            schema: SCHEMA_IDENTIFIER.to_string(),
            schema_version: SCHEMA_VERSION.to_string(),
            global_event_id: self.global_event_id,
            session_id: self.session_id,
            session_event_id: self.session_event_id,
            timestamp: self.timestamp,
            action_type: self.action_type,
            confidence: self.confidence,
            target: self.target,
            context: self.context,
            before: self.before,
            parameters: self.parameters,
            after: self.after,
            evidence: self.evidence,
            state_evidence: self.state_evidence,
            duration_ms: self.duration_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonical_action_36_types_serde() {
        let action_types = vec![
            ActionType::Click,
            ActionType::DoubleClick,
            ActionType::RightClick,
            ActionType::MiddleClick,
            ActionType::DragDrop,
            ActionType::Scroll,
            ActionType::TypeText,
            ActionType::KeyPress,
            ActionType::Shortcut,
            ActionType::Copy,
            ActionType::Cut,
            ActionType::Paste,
            ActionType::WindowSwitch,
            ActionType::WindowOpen,
            ActionType::WindowClose,
            ActionType::AppOpen,
            ActionType::AppClose,
            ActionType::Navigate,
            ActionType::FileOpen,
            ActionType::FileSave,
            ActionType::FileSaveAs,
            ActionType::FileCreate,
            ActionType::FileCopy,
            ActionType::FileMove,
            ActionType::FileRename,
            ActionType::FileDelete,
            ActionType::FileUpload,
            ActionType::FileDownload,
            ActionType::FileExport,
            ActionType::DialogOpen,
            ActionType::DialogConfirm,
            ActionType::DialogCancel,
            ActionType::Wait,
            ActionType::UserIdle,
            ActionType::SystemLock,
            ActionType::SystemUnlock,
            ActionType::SystemSleep,
            ActionType::SystemResume,
            ActionType::UnknownInteraction,
        ];

        assert!(action_types.len() >= 36);

        for at in action_types {
            let json = serde_json::to_string(&at).unwrap();
            let deserialized: ActionType = serde_json::from_str(&json).unwrap();
            assert_eq!(at, deserialized);
        }
    }

    #[test]
    fn test_builder_clamping_and_construction() {
        let ts = DualTimestamp::from_parts(chrono::Utc::now(), 100, 0);
        let action = CanonicalActionBuilder::new(
            GlobalEventId::new(42),
            SessionId::new("session_1"),
            1,
            ts,
            ActionType::Click,
            ActionParameters::Click(ClickParams::default()),
        )
        .confidence(1.5) // Clamped to 1.0
        .duration_ms(25)
        .build();

        assert_eq!(action.confidence, 1.0);
        assert_eq!(action.duration_ms, Some(25));
        assert_eq!(action.schema, SCHEMA_IDENTIFIER);
        assert_eq!(action.schema_version, SCHEMA_VERSION);
    }
}
