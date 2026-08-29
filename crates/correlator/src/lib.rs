//! Canonical action builder, typing/scroll burst grouping, and confidence scoring.

pub mod drag_drop;
pub mod engine;
pub mod scroll;
pub mod typing;

pub use drag_drop::DragDropStateMachine;
pub use engine::CorrelationEngine;
pub use scroll::ScrollBurstAggregator;
pub use typing::TypingBurstAggregator;

#[cfg(test)]
mod tests {
    use super::*;
    use core_types::action::{ActionParameters, ActionType};
    use core_types::event::{
        EventSource, RawEvent, RawEventPayload,
        RawKeyboardEvent, RawMouseEvent,
    };
    use core_types::id::{GlobalEventId, SessionId};
    use core_types::metadata::{MouseButton, Point2D, TargetMetadata};
    use core_types::timestamp::DualTimestamp;
    use std::sync::atomic::AtomicU64;
    use std::sync::Arc;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn test_click_action_generation() {
        let global_seq = Arc::new(AtomicU64::new(100));
        let mut engine = CorrelationEngine::new("sess_test", "user1", "mach1", global_seq);

        let mouse_event = RawMouseEvent {
            event_type: "CLICK".to_string(),
            button: MouseButton::Left,
            physical_x: 200,
            physical_y: 350,
            normalized_x: 0.1,
            normalized_y: 0.3,
            delta_x: 0.0,
            delta_y: 0.0,
            monitor_id: 1,
            coords: Point2D::new(200, 350, 0.1, 0.3),
            state: "DOWN".to_string(),
        };

        let raw = RawEvent::new(
            1,
            GlobalEventId::new(100),
            DualTimestamp::now(),
            "mach1".to_string(),
            1,
            "user1".to_string(),
            EventSource::Win32Hook,
            1,
            RawEventPayload::Mouse(mouse_event),
        );

        let target = TargetMetadata {
            name: Some("OKButton".to_string()),
            control_type: Some("Button".to_string()),
            ..Default::default()
        };

        let actions = engine.process_event(&raw, Some(target));
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action_type, ActionType::Click);
        assert_eq!(actions[0].target.name.as_deref(), Some("OKButton"));
    }

    #[test]
    fn mouse_down_then_up_emits_click_with_release_target() {
        let global_seq = Arc::new(AtomicU64::new(100));
        let mut engine = CorrelationEngine::new("sess_test", "user1", "mach1", global_seq);
        let timestamp = DualTimestamp::now();

        let down = RawEvent::new(
            1,
            GlobalEventId::new(100),
            timestamp,
            "mach1".to_string(),
            1,
            "user1".to_string(),
            EventSource::Win32Hook,
            1,
            RawEventPayload::Mouse(RawMouseEvent {
                event_type: "MOUSE_DOWN".to_string(),
                button: MouseButton::Left,
                physical_x: 200,
                physical_y: 350,
                normalized_x: 0.1,
                normalized_y: 0.3,
                delta_x: 0.0,
                delta_y: 0.0,
                monitor_id: 1,
                coords: Point2D::new(200, 350, 0.1, 0.3),
                state: "DOWN".to_string(),
            }),
        );
        assert!(engine.process_event(&down, None).is_empty());

        let up = RawEvent::new(
            2,
            GlobalEventId::new(101),
            timestamp,
            "mach1".to_string(),
            1,
            "user1".to_string(),
            EventSource::Win32Hook,
            2,
            RawEventPayload::Mouse(RawMouseEvent {
                event_type: "MOUSE_UP".to_string(),
                button: MouseButton::Left,
                physical_x: 201,
                physical_y: 351,
                normalized_x: 0.101,
                normalized_y: 0.301,
                delta_x: 0.0,
                delta_y: 0.0,
                monitor_id: 1,
                coords: Point2D::new(201, 351, 0.101, 0.301),
                state: "UP".to_string(),
            }),
        );
        let target = TargetMetadata {
            name: Some("Save".to_string()),
            control_type: Some("Button".to_string()),
            ..Default::default()
        };

        let actions = engine.process_event(&up, Some(target));
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action_type, ActionType::Click);
        assert_eq!(actions[0].target.name.as_deref(), Some("Save"));
    }

    #[test]
    fn test_drag_drop_state_machine() {
        let global_seq = Arc::new(AtomicU64::new(200));
        let mut engine = CorrelationEngine::new("sess_test", "user1", "mach1", global_seq);

        let ts = DualTimestamp::now();

        // 1. Mouse Down at (100, 100)
        let down_raw = RawEvent::new(
            1,
            GlobalEventId::new(200),
            ts,
            "mach1".to_string(),
            1,
            "user1".to_string(),
            EventSource::Win32Hook,
            1,
            RawEventPayload::Mouse(RawMouseEvent {
                event_type: "MOUSE_DOWN".to_string(),
                button: MouseButton::Left,
                physical_x: 100,
                physical_y: 100,
                normalized_x: 0.1,
                normalized_y: 0.1,
                delta_x: 0.0,
                delta_y: 0.0,
                monitor_id: 1,
                coords: Point2D::new(100, 100, 0.1, 0.1),
                state: "DOWN".to_string(),
            }),
        );
        let actions = engine.process_event(&down_raw, None);
        assert_eq!(actions.len(), 0);

        // 2. Mouse Move to (150, 150) (dist ~ 70.7px > 5px)
        let move_raw = RawEvent::new(
            2,
            GlobalEventId::new(201),
            ts,
            "mach1".to_string(),
            1,
            "user1".to_string(),
            EventSource::Win32Hook,
            2,
            RawEventPayload::Mouse(RawMouseEvent {
                event_type: "MOUSE_MOVE".to_string(),
                button: MouseButton::None,
                physical_x: 150,
                physical_y: 150,
                normalized_x: 0.15,
                normalized_y: 0.15,
                delta_x: 0.0,
                delta_y: 0.0,
                monitor_id: 1,
                coords: Point2D::new(150, 150, 0.15, 0.15),
                state: "MOVE".to_string(),
            }),
        );
        let actions = engine.process_event(&move_raw, None);
        assert_eq!(actions.len(), 0);

        // 3. Mouse Up at (150, 150) -> triggers DRAG_DROP
        let up_raw = RawEvent::new(
            3,
            GlobalEventId::new(202),
            ts,
            "mach1".to_string(),
            1,
            "user1".to_string(),
            EventSource::Win32Hook,
            3,
            RawEventPayload::Mouse(RawMouseEvent {
                event_type: "MOUSE_UP".to_string(),
                button: MouseButton::Left,
                physical_x: 150,
                physical_y: 150,
                normalized_x: 0.15,
                normalized_y: 0.15,
                delta_x: 0.0,
                delta_y: 0.0,
                monitor_id: 1,
                coords: Point2D::new(150, 150, 0.15, 0.15),
                state: "UP".to_string(),
            }),
        );
        let actions = engine.process_event(&up_raw, None);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action_type, ActionType::DragDrop);
        if let ActionParameters::DragDrop(ref dd) = actions[0].parameters {
            assert!(dd.distance_px > 70.0);
            assert_eq!(dd.start_coords.physical_x, 100);
            assert_eq!(dd.end_coords.physical_x, 150);
        } else {
            panic!("Expected DragDrop parameters");
        }
    }

    #[test]
    fn test_typing_burst_aggregation() {
        let global_seq = Arc::new(AtomicU64::new(300));
        let mut engine = CorrelationEngine::new("sess_test", "user1", "mach1", global_seq);

        let keys = [('H', 0x48), ('e', 0x45), ('l', 0x4C), ('l', 0x4C), ('o', 0x4F)];
        for (idx, &(ch, vk)) in keys.iter().enumerate() {
            let raw = RawEvent::new(
                idx as u64 + 1,
                GlobalEventId::new(300 + idx as u64),
                DualTimestamp::now(),
                "mach1".to_string(),
                1,
                "user1".to_string(),
                EventSource::Win32Hook,
                idx as u64 + 1,
                RawEventPayload::Keyboard(RawKeyboardEvent {
                    event_type: "KEY_DOWN".to_string(),
                    vk_code: vk,
                    scan_code: 0,
                    key_name: ch.to_string(),
                    is_injected: false,
                    modifiers: Default::default(),
                }),
            );
            let _ = engine.process_event(&raw, None);
        }

        // Force flush
        let _flushed = engine.periodic_flush();
        // Since timeout hasn't elapsed naturally, test forced flush on typing aggregator directly
        let mut aggregator = TypingBurstAggregator::new(Duration::from_millis(50));
        aggregator.on_keystroke(
            DualTimestamp::now(),
            0x48,
            "H",
            true,
            Default::default(),
            Default::default(),
            &SessionId::new("sess_1"),
            400,
            1,
        );
        aggregator.on_keystroke(
            DualTimestamp::now(),
            0x49,
            "i",
            true,
            Default::default(),
            Default::default(),
            &SessionId::new("sess_1"),
            401,
            2,
        );

        sleep(Duration::from_millis(60));
        let action = aggregator
            .check_timeout(&SessionId::new("sess_1"), 402, 3)
            .expect("Typing burst completed");

        assert_eq!(action.action_type, ActionType::TypeText);
        if let ActionParameters::TypeText(ref tp) = action.parameters {
            assert_eq!(tp.text, "Hi");
            assert_eq!(tp.character_count, 2);
        }
    }
}
