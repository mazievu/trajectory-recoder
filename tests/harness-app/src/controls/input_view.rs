use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputKind {
    SingleLine,
    MultiLine,
    Password,
    Numeric,
    CreditCard,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputControl {
    pub automation_id: String,
    pub name: String,
    pub kind: InputKind,
    pub text: String,
    pub is_password: bool,
    pub is_read_only: bool,
    pub placeholder: String,
}

impl InputControl {
    pub fn new(automation_id: impl Into<String>, name: impl Into<String>, kind: InputKind) -> Self {
        let is_password = kind == InputKind::Password;
        Self {
            automation_id: automation_id.into(),
            name: name.into(),
            kind,
            text: String::new(),
            is_password,
            is_read_only: false,
            placeholder: String::new(),
        }
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        if !self.is_read_only {
            self.text = text.into();
        }
    }

    pub fn append_text(&mut self, text: &str) {
        if !self.is_read_only {
            self.text.push_str(text);
        }
    }

    pub fn clear(&mut self) {
        if !self.is_read_only {
            self.text.clear();
        }
    }
}
