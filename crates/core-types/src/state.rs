use crate::metadata::WindowContext;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct StateSnapshot {
    pub screenshot: Option<ScreenshotRef>,
    pub ui_state: Option<UiStateSnapshot>,
    pub active_window: Option<WindowContext>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScreenshotRef {
    pub file_name: String,
    pub relative_path: String,
    pub sha256: String,
    pub width: u32,
    pub height: u32,
    pub format: String, // "image/webp"
    pub trigger: ScreenshotTrigger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScreenshotTrigger {
    #[default]
    BeforeAction,
    AfterAction,
    StabilizedAfter200Ms,
    StabilizedAfter500Ms,
    StabilizedAfter1000Ms,
    Periodic,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct UiStateSnapshot {
    pub focused_element_name: Option<String>,
    pub focused_control_type: Option<String>,
    pub focused_automation_id: Option<String>,
    pub modal_active: bool,
    pub progress_indicator_active: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ActionEvidence {
    pub raw_event_ids: Vec<u64>,
    pub video_ranges: Vec<VideoTimeRange>,
    pub screenshot_refs: Vec<ScreenshotRef>,
    pub state_changes: Vec<StateChange>,
}

pub type StateEvidence = ActionEvidence;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoTimeRange {
    pub video_file: String,
    pub start_pts_ns: u64,
    pub end_pts_ns: u64,
    pub start_wall_utc: DateTime<Utc>,
    pub end_wall_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateChange {
    pub kind: StateChangeKind,
    pub description: String,
    pub target: Option<String>,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StateChangeKind {
    DialogAppeared,
    DialogDisappeared,
    ModalAppeared,
    Toast,
    ErrorNotification,
    LoadingStarted,
    LoadingEnded,
    PageNavigation,
    WindowAppeared,
    WindowDisappeared,
    FileCreated,
    DownloadCompleted,
    Custom(String),
}
