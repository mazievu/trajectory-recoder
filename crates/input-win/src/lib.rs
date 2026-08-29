//! Win32 low-level mouse and keyboard hooks with non-blocking message loop
//! and simulation support for headless CI test environments.

pub mod coordinate;
pub mod double_click;
pub mod hook;
pub mod keyboard_state;
pub mod manager;

pub use coordinate::{CoordinateMapper, MonitorBounds};
pub use double_click::DoubleClickDetector;
pub use keyboard_state::KeyboardModifierTracker;
pub use manager::InputHookManager;

#[cfg(test)]
mod tests {
    use super::*;
    use core_types::event::RawEventPayload;
    use core_types::metadata::MouseButton;
    use std::time::Duration;

    #[test]
    fn test_coordinate_normalization_single_and_multi_monitor() {
        let mon1 = MonitorBounds::new(0, 0, 0, 1920, 1080, true);
        let mon2 = MonitorBounds::new(1, 1920, 0, 1920, 1080, false);
        let mapper = CoordinateMapper::with_monitors(vec![mon1, mon2]);

        // Point on primary monitor
        let (mon_id, nx, ny, pt) = mapper.map_point(960, 540);
        assert_eq!(mon_id, 0);
        assert!((nx - 0.5).abs() < 1e-4);
        assert!((ny - 0.5).abs() < 1e-4);
        assert_eq!(pt.physical_x, 960);
        assert_eq!(pt.physical_y, 540);

        // Point on second monitor
        let (mon_id2, nx2, ny2, pt2) = mapper.map_point(2880, 540);
        assert_eq!(mon_id2, 1);
        assert!((nx2 - 0.5).abs() < 1e-4);
        assert!((ny2 - 0.5).abs() < 1e-4);
        assert_eq!(pt2.physical_x, 2880);
        assert_eq!(pt2.physical_y, 540);
    }

    #[test]
    fn test_double_click_detection() {
        let mut detector = DoubleClickDetector::new(500, 4);

        // First click
        assert!(!detector.check_and_update(MouseButton::Left, 100, 100, 1000));
        // Quick second click at same location -> Double Click
        assert!(detector.check_and_update(MouseButton::Left, 102, 101, 1200));

        // Third click -> resets to single click
        assert!(!detector.check_and_update(MouseButton::Left, 100, 100, 1300));

        // Slow click (> 500ms) -> Not double click
        assert!(!detector.check_and_update(MouseButton::Left, 100, 100, 2000));

        // Far click (> 4px) -> Not double click
        assert!(!detector.check_and_update(MouseButton::Left, 100, 100, 3000));
        assert!(!detector.check_and_update(MouseButton::Left, 120, 100, 3100));
    }

    #[test]
    fn test_keyboard_modifier_tracker() {
        let mut tracker = KeyboardModifierTracker::new();
        assert!(!tracker.current_modifiers().ctrl);
        assert!(!tracker.current_modifiers().shift);

        // Press Left Ctrl (0xA2)
        tracker.update_vk(0xA2, true);
        assert!(tracker.current_modifiers().ctrl);

        // Press Shift (0x10)
        tracker.update_vk(0x10, true);
        assert!(tracker.current_modifiers().shift);
        assert!(tracker.current_modifiers().ctrl);

        // Release Left Ctrl
        tracker.update_vk(0xA2, false);
        assert!(!tracker.current_modifiers().ctrl);
        assert!(tracker.current_modifiers().shift);

        // CapsLock toggle
        tracker.update_vk(0x14, true);
        assert!(tracker.current_modifiers().caps_lock);
        tracker.update_vk(0x14, true);
        assert!(!tracker.current_modifiers().caps_lock);
    }

    #[test]
    fn test_key_name_translation() {
        assert_eq!(KeyboardModifierTracker::vk_to_key_name(0x0D), "Enter");
        assert_eq!(KeyboardModifierTracker::vk_to_key_name(0x1B), "Escape");
        assert_eq!(KeyboardModifierTracker::vk_to_key_name(0x09), "Tab");
        assert_eq!(KeyboardModifierTracker::vk_to_key_name(0x20), "Space");
        assert_eq!(KeyboardModifierTracker::vk_to_key_name(0x41), "A");
        assert_eq!(KeyboardModifierTracker::vk_to_key_name(0x30), "0");
        assert_eq!(KeyboardModifierTracker::vk_to_key_name(0x70), "F1");
    }

    #[test]
    fn test_input_hook_manager_simulation_pipeline() {
        let mgr = InputHookManager::start_mock("test_pc", 1, "test_user");
        let rx = mgr.receiver();

        mgr.simulate_mouse_move(100, 200);
        mgr.simulate_mouse_down(MouseButton::Left, 100, 200);
        mgr.simulate_mouse_up(MouseButton::Left, 100, 200);
        mgr.simulate_mouse_wheel(100, 200, 0.0, 120.0);
        mgr.simulate_key_down(0x41, 0x1E); // 'A'
        mgr.simulate_key_up(0x41, 0x1E);

        let e1 = rx.recv_timeout(Duration::from_millis(100)).unwrap();
        if let RawEventPayload::Mouse(m) = e1.payload {
            assert_eq!(m.event_type, "MOUSE_MOVE");
            assert_eq!(m.physical_x, 100);
            assert_eq!(m.physical_y, 200);
        } else {
            panic!("Expected Mouse payload");
        }

        let e2 = rx.recv_timeout(Duration::from_millis(100)).unwrap();
        if let RawEventPayload::Mouse(m) = e2.payload {
            assert_eq!(m.event_type, "MOUSE_DOWN");
            assert_eq!(m.button, MouseButton::Left);
        } else {
            panic!("Expected Mouse payload");
        }

        let e3 = rx.recv_timeout(Duration::from_millis(100)).unwrap();
        if let RawEventPayload::Mouse(m) = e3.payload {
            assert_eq!(m.event_type, "MOUSE_UP");
        } else {
            panic!("Expected Mouse payload");
        }

        let e4 = rx.recv_timeout(Duration::from_millis(100)).unwrap();
        if let RawEventPayload::Mouse(m) = e4.payload {
            assert_eq!(m.event_type, "MOUSE_WHEEL");
            assert_eq!(m.delta_y, 120.0);
        } else {
            panic!("Expected Mouse payload");
        }

        let e5 = rx.recv_timeout(Duration::from_millis(100)).unwrap();
        if let RawEventPayload::Keyboard(k) = e5.payload {
            assert_eq!(k.event_type, "KEY_DOWN");
            assert_eq!(k.vk_code, 0x41);
            assert_eq!(k.key_name, "A");
        } else {
            panic!("Expected Keyboard payload");
        }

        let e6 = rx.recv_timeout(Duration::from_millis(100)).unwrap();
        if let RawEventPayload::Keyboard(k) = e6.payload {
            assert_eq!(k.event_type, "KEY_UP");
            assert_eq!(k.vk_code, 0x41);
        } else {
            panic!("Expected Keyboard payload");
        }
    }
}
