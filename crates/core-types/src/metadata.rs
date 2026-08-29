use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MouseButton {
    #[default]
    None,
    Left,
    Right,
    Middle,
    X1,
    X2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct ModifierState {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub win: bool,
    pub caps_lock: bool,
    pub num_lock: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScrollDirection {
    VerticalUp,
    VerticalDown,
    HorizontalLeft,
    HorizontalRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct Point2D {
    pub physical_x: i32,
    pub physical_y: i32,
    pub normalized_x: f32,
    pub normalized_y: f32,
}

impl Point2D {
    pub const fn new(
        physical_x: i32,
        physical_y: i32,
        normalized_x: f32,
        normalized_y: f32,
    ) -> Self {
        Self {
            physical_x,
            physical_y,
            normalized_x,
            normalized_y,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct BoundingRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub width: u32,
    pub height: u32,
}

impl BoundingRect {
    pub fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        let width = (right.saturating_sub(left)).max(0) as u32;
        let height = (bottom.saturating_sub(top)).max(0) as u32;
        Self {
            left,
            top,
            right,
            bottom,
            width,
            height,
        }
    }

    pub fn to_bounding_box(&self) -> BoundingBox {
        BoundingBox {
            x: self.left,
            y: self.top,
            width: self.width,
            height: self.height,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct BoundingBox {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl BoundingBox {
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x
            && px <= (self.x.saturating_add(self.width as i32))
            && py >= self.y
            && py <= (self.y.saturating_add(self.height as i32))
    }

    pub fn to_bounding_rect(&self) -> BoundingRect {
        BoundingRect {
            left: self.x,
            top: self.y,
            right: self.x.saturating_add(self.width as i32),
            bottom: self.y.saturating_add(self.height as i32),
            width: self.width,
            height: self.height,
        }
    }
}

/// Comprehensive semantic metadata of the interacted UI element.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TargetMetadata {
    pub name: Option<String>,
    pub control_type: Option<String>,
    pub automation_id: Option<String>,
    pub class_name: Option<String>,
    pub framework_id: Option<String>,
    pub bounding_rect: Option<BoundingRect>,
    pub bounding_box: Option<BoundingBox>,
    pub is_enabled: Option<bool>,
    pub is_keyboard_focusable: Option<bool>,
    pub is_password: bool,
    pub value: Option<String>,
    pub help_text: Option<String>,
    pub ancestor_chain: Vec<AncestorElementMetadata>,
    pub ancestors: Vec<TargetMetadata>,
    pub dom_selector: Option<DomSelectorMetadata>,
    pub xpath: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AncestorElementMetadata {
    pub level: u32, // 1 = Parent, 2 = Grandparent, 3 = Great-Grandparent
    pub name: Option<String>,
    pub control_type: Option<String>,
    pub automation_id: Option<String>,
    pub class_name: Option<String>,
    pub framework_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DomSelectorMetadata {
    pub tag: String,
    pub role: Option<String>,
    pub visible_text: Option<String>,
    pub aria_label: Option<String>,
    pub id: Option<String>,
    pub class: Option<String>,
    pub href: Option<String>,
    pub placeholder: Option<String>,
    pub input_type: Option<String>,
    pub css_selector: Option<String>,
    pub xpath: Option<String>,
}

/// Environment, application, window, and display context at action time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ContextMetadata {
    pub application: ApplicationContext,
    pub window: WindowContext,
    pub browser: Option<BrowserContext>,
    pub display: DisplayContext,
    pub user_id: String,
    pub machine_id: String,
    // Context fields for flat access
    pub process_name: String,
    pub process_id: u32,
    pub executable_path: String,
    pub window_title: String,
    pub window_handle: u64,
    pub monitor_id: u32,
    pub is_fullscreen: bool,
    pub is_elevated: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ApplicationContext {
    pub process_name: String,
    pub pid: u32,
    pub executable_path: Option<String>,
    pub app_id: Option<String>,
    pub is_elevated: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct WindowContext {
    pub hwnd: u64,
    pub title: String,
    pub bounds: BoundingRect,
    pub is_maximized: bool,
    pub is_minimized: bool,
    pub is_foreground: bool,
    pub is_fullscreen: bool,
    pub dpi: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct BrowserContext {
    pub browser_family: String, // "Chrome", "Edge"
    pub tab_id: u32,
    pub url: String,
    pub page_title: String,
    pub domain: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DisplayContext {
    pub active_monitor_id: u32,
    pub monitor_count: u32,
    pub primary_resolution_width: u32,
    pub primary_resolution_height: u32,
    pub virtual_screen_bounds: BoundingRect,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounding_box_geometry() {
        let bbox = BoundingBox::new(10, 20, 100, 50);
        assert!(bbox.contains(15, 25));
        assert!(bbox.contains(10, 20));
        assert!(bbox.contains(110, 70));
        assert!(!bbox.contains(5, 5));
        assert!(!bbox.contains(120, 80));

        let rect = bbox.to_bounding_rect();
        assert_eq!(rect.left, 10);
        assert_eq!(rect.top, 20);
        assert_eq!(rect.right, 110);
        assert_eq!(rect.bottom, 70);
        assert_eq!(rect.width, 100);
        assert_eq!(rect.height, 50);
    }
}
