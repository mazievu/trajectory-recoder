use crate::model::{UiaAncestorInfo, UiaElementInfo};
use core_types::metadata::BoundingRect;
use parking_lot::RwLock;
use std::sync::Arc;

/// In-memory mock UIA element repository for testing without an active Windows display session.
#[derive(Debug, Clone, Default)]
pub struct MockUiaStore {
    elements: Arc<RwLock<Vec<UiaElementInfo>>>,
    focused: Arc<RwLock<Option<UiaElementInfo>>>,
}

impl MockUiaStore {
    pub fn new() -> Self {
        Self {
            elements: Arc::new(RwLock::new(Vec::new())),
            focused: Arc::new(RwLock::new(None)),
        }
    }

    pub fn add_element(&self, elem: UiaElementInfo) {
        self.elements.write().push(elem);
    }

    pub fn set_focused(&self, elem: Option<UiaElementInfo>) {
        *self.focused.write() = elem;
    }

    pub fn find_at_point(&self, x: i32, y: i32) -> Option<UiaElementInfo> {
        let lock = self.elements.read();
        // Return most deeply nested element (smallest rect containing point or last added)
        for elem in lock.iter().rev() {
            if let Some(rect) = elem.bounding_rect {
                if x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom {
                    return Some(elem.clone());
                }
            }
        }
        None
    }

    pub fn get_focused(&self) -> Option<UiaElementInfo> {
        self.focused.read().clone()
    }
}

/// Helper to create a standard mock button with a 3-level ancestor tree.
pub fn create_mock_button(
    name: &str,
    automation_id: &str,
    left: i32,
    top: i32,
    width: u32,
    height: u32,
) -> UiaElementInfo {
    UiaElementInfo {
        name: Some(name.to_string()),
        control_type: "Button".to_string(),
        control_type_id: 50000,
        automation_id: Some(automation_id.to_string()),
        class_name: Some("WpfButton".to_string()),
        framework_id: Some("WPF".to_string()),
        bounding_rect: Some(BoundingRect::new(
            left,
            top,
            left + width as i32,
            top + height as i32,
        )),
        is_enabled: true,
        is_keyboard_focusable: true,
        is_password: false,
        is_offscreen: false,
        value: None,
        help_text: Some(format!("Help for {}", name)),
        ancestors: vec![
            UiaAncestorInfo {
                level: 1,
                name: Some("ToolBarPanel".to_string()),
                control_type: "ToolBar".to_string(),
                control_type_id: 50021,
                automation_id: Some("MainToolBar".to_string()),
                class_name: Some("ToolBar".to_string()),
                framework_id: Some("WPF".to_string()),
            },
            UiaAncestorInfo {
                level: 2,
                name: Some("RibbonGroup".to_string()),
                control_type: "Group".to_string(),
                control_type_id: 50026,
                automation_id: Some("HomeRibbon".to_string()),
                class_name: Some("RibbonGroup".to_string()),
                framework_id: Some("WPF".to_string()),
            },
            UiaAncestorInfo {
                level: 3,
                name: Some("MainWindow".to_string()),
                control_type: "Window".to_string(),
                control_type_id: 50032,
                automation_id: Some("AppWindow".to_string()),
                class_name: Some("Window".to_string()),
                framework_id: Some("WPF".to_string()),
            },
        ],
    }
}

/// Helper to create a mock password input box.
pub fn create_mock_password_box(
    name: &str,
    automation_id: &str,
    left: i32,
    top: i32,
    width: u32,
    height: u32,
) -> UiaElementInfo {
    let mut elem = create_mock_button(name, automation_id, left, top, width, height);
    elem.control_type = "Edit".to_string();
    elem.control_type_id = 50004;
    elem.is_password = true;
    elem.value = Some("Secret123!".to_string());
    elem
}
