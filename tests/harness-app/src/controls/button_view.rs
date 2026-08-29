use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ButtonKind {
    Push,
    Toggle,
    Radio,
    Submit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonControl {
    pub automation_id: String,
    pub name: String,
    pub kind: ButtonKind,
    pub is_checked: bool,
    pub is_enabled: bool,
    pub click_count: u64,
}

impl ButtonControl {
    pub fn new(automation_id: impl Into<String>, name: impl Into<String>, kind: ButtonKind) -> Self {
        Self {
            automation_id: automation_id.into(),
            name: name.into(),
            kind,
            is_checked: false,
            is_enabled: true,
            click_count: 0,
        }
    }

    pub fn click(&mut self) -> bool {
        if !self.is_enabled {
            return false;
        }
        self.click_count += 1;
        match self.kind {
            ButtonKind::Toggle => {
                self.is_checked = !self.is_checked;
            }
            ButtonKind::Radio => {
                self.is_checked = true;
            }
            ButtonKind::Push | ButtonKind::Submit => {}
        }
        true
    }
}
