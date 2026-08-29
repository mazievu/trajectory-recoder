use serde::{Deserialize, Serialize};

/// Telemetry event capturing user interaction with standard Windows file dialogs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileDialogEvent {
    pub dialog_type: String, // "OPEN", "SAVE", "SAVE_AS", "EXPORT", "UPLOAD"
    pub dialog_title: String,
    pub selected_path: Option<String>,
    pub filter_format: Option<String>,
    pub process_name: String,
    pub is_confirmed: bool,
}

impl FileDialogEvent {
    pub fn new(
        dialog_type: impl Into<String>,
        dialog_title: impl Into<String>,
        selected_path: Option<String>,
        filter_format: Option<String>,
        process_name: impl Into<String>,
        is_confirmed: bool,
    ) -> Self {
        Self {
            dialog_type: dialog_type.into(),
            dialog_title: dialog_title.into(),
            selected_path,
            filter_format,
            process_name: process_name.into(),
            is_confirmed,
        }
    }
}
