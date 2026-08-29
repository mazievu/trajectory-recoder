use core_types::{ModifierState, MouseButton, Point2D, RawKeyboardEvent, RawMouseEvent};

pub struct FakeInputDriver;

impl FakeInputDriver {
    pub fn fake_mouse_move(x: i32, y: i32) -> RawMouseEvent {
        RawMouseEvent {
            event_type: "MOUSE_MOVE".to_string(),
            button: MouseButton::None,
            coords: Point2D::new(x, y, x as f32 / 1920.0, y as f32 / 1080.0),
            monitor_id: 0,
            delta_x: 0.0,
            delta_y: 0.0,
            state: "move".to_string(),
            physical_x: x,
            physical_y: y,
            normalized_x: x as f32 / 1920.0,
            normalized_y: y as f32 / 1080.0,
        }
    }

    pub fn fake_mouse_click(x: i32, y: i32, button: MouseButton) -> (RawMouseEvent, RawMouseEvent) {
        let down = RawMouseEvent {
            event_type: "MOUSE_DOWN".to_string(),
            button,
            coords: Point2D::new(x, y, x as f32 / 1920.0, y as f32 / 1080.0),
            monitor_id: 0,
            delta_x: 0.0,
            delta_y: 0.0,
            state: "down".to_string(),
            physical_x: x,
            physical_y: y,
            normalized_x: x as f32 / 1920.0,
            normalized_y: y as f32 / 1080.0,
        };
        let up = RawMouseEvent {
            event_type: "MOUSE_UP".to_string(),
            button,
            coords: Point2D::new(x, y, x as f32 / 1920.0, y as f32 / 1080.0),
            monitor_id: 0,
            delta_x: 0.0,
            delta_y: 0.0,
            state: "up".to_string(),
            physical_x: x,
            physical_y: y,
            normalized_x: x as f32 / 1920.0,
            normalized_y: y as f32 / 1080.0,
        };
        (down, up)
    }

    pub fn fake_key_stroke(
        key_name: &str,
        vk_code: u32,
        modifiers: ModifierState,
    ) -> (RawKeyboardEvent, RawKeyboardEvent) {
        let down = RawKeyboardEvent {
            event_type: "KEY_DOWN".to_string(),
            vk_code,
            scan_code: 0,
            key_name: key_name.to_string(),
            modifiers,
            is_injected: true,
        };
        let up = RawKeyboardEvent {
            event_type: "KEY_UP".to_string(),
            vk_code,
            scan_code: 0,
            key_name: key_name.to_string(),
            modifiers,
            is_injected: true,
        };
        (down, up)
    }
}
