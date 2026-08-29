use core_types::{ActionType, MouseButton, Point2D};
use proptest::prelude::*;

pub fn point2d_strategy() -> impl Strategy<Value = Point2D> {
    (0..1920i32, 0..1080i32).prop_map(|(x, y)| {
        Point2D::new(x, y, x as f32 / 1920.0, y as f32 / 1080.0)
    })
}

pub fn mouse_button_strategy() -> impl Strategy<Value = MouseButton> {
    prop_oneof![
        Just(MouseButton::Left),
        Just(MouseButton::Right),
        Just(MouseButton::Middle),
        Just(MouseButton::X1),
        Just(MouseButton::X2),
    ]
}

pub fn action_type_strategy() -> impl Strategy<Value = ActionType> {
    prop_oneof![
        Just(ActionType::Click),
        Just(ActionType::DoubleClick),
        Just(ActionType::RightClick),
        Just(ActionType::TypeText),
        Just(ActionType::KeyPress),
        Just(ActionType::Shortcut),
        Just(ActionType::Scroll),
        Just(ActionType::DragDrop),
        Just(ActionType::Copy),
        Just(ActionType::Paste),
    ]
}
