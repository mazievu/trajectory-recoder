use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiaBoundingRect {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiaControlType {
    Button,
    Edit,
    Text,
    CheckBox,
    RadioButton,
    ComboBox,
    List,
    ListItem,
    Pane,
    Window,
    Dialog,
    ProgressBar,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiaElementNode {
    pub automation_id: String,
    pub name: String,
    pub class_name: String,
    pub framework_id: String, // "Win32", "WPF", "WinUI"
    pub control_type: UiaControlType,
    pub bounding_box: UiaBoundingRect,
    pub is_password: bool,
    pub is_enabled: bool,
    pub is_offscreen: bool,
    pub children: Vec<UiaElementNode>,
}

impl UiaElementNode {
    pub fn new(
        automation_id: impl Into<String>,
        name: impl Into<String>,
        class_name: impl Into<String>,
        control_type: UiaControlType,
        rect: UiaBoundingRect,
    ) -> Self {
        Self {
            automation_id: automation_id.into(),
            name: name.into(),
            class_name: class_name.into(),
            framework_id: "Win32".to_string(),
            control_type,
            bounding_box: rect,
            is_password: false,
            is_enabled: true,
            is_offscreen: false,
            children: Vec::new(),
        }
    }

    pub fn find_by_automation_id(&self, id: &str) -> Option<&UiaElementNode> {
        if self.automation_id == id {
            return Some(self);
        }
        for child in &self.children {
            if let Some(found) = child.find_by_automation_id(id) {
                return Some(found);
            }
        }
        None
    }

    pub fn hit_test(&self, x: f64, y: f64) -> Option<&UiaElementNode> {
        if x >= self.bounding_box.left
            && x <= self.bounding_box.left + self.bounding_box.width
            && y >= self.bounding_box.top
            && y <= self.bounding_box.top + self.bounding_box.height
        {
            // Check children first (deepest hit)
            for child in self.children.iter().rev() {
                if let Some(hit) = child.hit_test(x, y) {
                    return Some(hit);
                }
            }
            return Some(self);
        }
        None
    }
}
