use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DialogKind {
    Information,
    Warning,
    Error,
    OpenFile,
    SaveFile,
    FolderPicker,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DialogResult {
    None,
    Ok,
    Cancel,
    Yes,
    No,
    FileSelected(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogControl {
    pub dialog_id: String,
    pub title: String,
    pub message: String,
    pub kind: DialogKind,
    pub is_open: bool,
    pub last_result: DialogResult,
}

impl DialogControl {
    pub fn new(dialog_id: impl Into<String>, title: impl Into<String>, kind: DialogKind) -> Self {
        Self {
            dialog_id: dialog_id.into(),
            title: title.into(),
            message: String::new(),
            kind,
            is_open: false,
            last_result: DialogResult::None,
        }
    }

    pub fn open(&mut self, message: impl Into<String>) {
        self.message = message.into();
        self.is_open = true;
        self.last_result = DialogResult::None;
    }

    pub fn close(&mut self, result: DialogResult) {
        self.is_open = false;
        self.last_result = result;
    }
}
