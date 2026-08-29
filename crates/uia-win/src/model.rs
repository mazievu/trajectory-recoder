use core_types::metadata::{AncestorElementMetadata, BoundingRect, TargetMetadata};
use serde::{Deserialize, Serialize};

/// Detailed element information queried from UIAutomation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct UiaElementInfo {
    pub name: Option<String>,
    pub control_type: String,
    pub control_type_id: i32,
    pub automation_id: Option<String>,
    pub class_name: Option<String>,
    pub framework_id: Option<String>,
    pub bounding_rect: Option<BoundingRect>,
    pub is_enabled: bool,
    pub is_keyboard_focusable: bool,
    pub is_password: bool,
    pub is_offscreen: bool,
    pub value: Option<String>,
    pub help_text: Option<String>,
    pub ancestors: Vec<UiaAncestorInfo>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct UiaAncestorInfo {
    pub level: u32,
    pub name: Option<String>,
    pub control_type: String,
    pub control_type_id: i32,
    pub automation_id: Option<String>,
    pub class_name: Option<String>,
    pub framework_id: Option<String>,
}

impl UiaElementInfo {
    /// Convert `UiaElementInfo` into the canonical `TargetMetadata` schema.
    pub fn to_target_metadata(&self) -> TargetMetadata {
        let ancestor_chain = self
            .ancestors
            .iter()
            .map(|a| AncestorElementMetadata {
                level: a.level,
                name: a.name.clone(),
                control_type: Some(a.control_type.clone()),
                automation_id: a.automation_id.clone(),
                class_name: a.class_name.clone(),
                framework_id: a.framework_id.clone(),
            })
            .collect();

        TargetMetadata {
            name: self.name.clone(),
            control_type: Some(self.control_type.clone()),
            automation_id: self.automation_id.clone(),
            class_name: self.class_name.clone(),
            framework_id: self.framework_id.clone(),
            bounding_rect: self.bounding_rect,
            bounding_box: self.bounding_rect.map(|r| r.to_bounding_box()),
            is_enabled: Some(self.is_enabled),
            is_keyboard_focusable: Some(self.is_keyboard_focusable),
            is_password: self.is_password,
            value: if self.is_password {
                Some("[PASSWORD_REDACTED]".to_string())
            } else {
                self.value.clone()
            },
            help_text: self.help_text.clone(),
            ancestor_chain,
            ancestors: Vec::new(),
            dom_selector: None,
            xpath: None,
        }
    }
}

/// Map Win32 UIA Control Type ID to canonical string representation.
pub fn control_type_id_to_name(id: i32) -> &'static str {
    match id {
        50000 => "Button",
        50001 => "Calendar",
        50002 => "CheckBox",
        50003 => "ComboBox",
        50004 => "Edit",
        50005 => "Hyperlink",
        50006 => "Image",
        50007 => "ListItem",
        50008 => "List",
        50009 => "Menu",
        50010 => "MenuBar",
        50011 => "MenuItem",
        50012 => "ProgressBar",
        50013 => "RadioButton",
        50014 => "ScrollBar",
        50015 => "Slider",
        50016 => "Spinner",
        50017 => "StatusBar",
        50018 => "Tab",
        50019 => "TabItem",
        50020 => "Text",
        50021 => "ToolBar",
        50022 => "ToolTip",
        50023 => "Tree",
        50024 => "TreeItem",
        50025 => "Custom",
        50026 => "Group",
        50027 => "Thumb",
        50028 => "DataGrid",
        50029 => "DataItem",
        50030 => "Document",
        50031 => "SplitButton",
        50032 => "Window",
        50033 => "Pane",
        50034 => "Header",
        50035 => "HeaderItem",
        50036 => "Table",
        50037 => "TitleBar",
        50038 => "Separator",
        50039 => "SemanticZoom",
        50040 => "AppBar",
        _ => "Unknown",
    }
}
