//! Canonical action builder, typing/scroll burst grouping, and confidence scoring.

pub mod drag_drop;
pub mod engine;
pub mod mouse_move;
pub mod raw_aggregator;
pub mod scroll;
pub mod typing;

pub use drag_drop::DragDropStateMachine;
pub use engine::CorrelationEngine;
pub use mouse_move::MouseMoveAggregator;
pub use raw_aggregator::RawEventAggregator;
pub use scroll::ScrollBurstAggregator;
pub use typing::TypingBurstAggregator;

#[cfg(test)]
mod tests {
    use super::*;
    use core_types::action::{ActionParameters, ActionType};
    use core_types::event::{
        EventSource, RawBrowserEvent, RawEvent, RawEventPayload, RawKeyboardEvent, RawMouseEvent,
        RawWindowEvent,
    };
    use core_types::id::{GlobalEventId, SessionId};
    use core_types::metadata::{BoundingRect, MouseButton, Point2D, TargetMetadata};
    use core_types::timestamp::DualTimestamp;
    use std::sync::Arc;
    use std::sync::atomic::AtomicU64;
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
    fn window_move_does_not_emit_a_canonical_action() {
        let global_seq = Arc::new(AtomicU64::new(100));
        let mut engine = CorrelationEngine::new("sess_test", "user1", "mach1", global_seq);
        let raw = RawEvent::new(
            1,
            GlobalEventId::new(100),
            DualTimestamp::now(),
            "mach1".to_string(),
            1,
            "user1".to_string(),
            EventSource::WinEvent,
            1,
            RawEventPayload::Window(RawWindowEvent {
                event_type: "MOVE".to_string(),
                hwnd: 1,
                pid: 100,
                process_name: "notepad.exe".to_string(),
                window_title: "Untitled - Notepad".to_string(),
                bounds: BoundingRect::new(0, 0, 100, 100),
                monitor_id: 1,
                dpi: 96,
            }),
        );

        assert!(engine.process_event(&raw, None).is_empty());
    }

    #[test]
    fn browser_navigation_emits_canonical_navigate_action() {
        let global_seq = Arc::new(AtomicU64::new(100));
        let mut engine = CorrelationEngine::new("sess_test", "user1", "mach1", global_seq);
        let raw = RawEvent::new(
            1,
            GlobalEventId::new(100),
            DualTimestamp::now(),
            "mach1".to_string(),
            1,
            "user1".to_string(),
            EventSource::BrowserExtension,
            1,
            RawEventPayload::Browser(RawBrowserEvent {
                tab_id: 7,
                event_type: "SPA_NAVIGATION".to_string(),
                url: "https://example.test/orders".to_string(),
                tag_name: "body".to_string(),
                target_id: None,
                target_class: None,
                target_text: None,
                css_selector: None,
                xpath: None,
                bounds: Default::default(),
            }),
        );

        let actions = engine.process_event(&raw, None);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action_type, ActionType::Navigate);
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

        let keys = [
            ('H', 0x48),
            ('e', 0x45),
            ('l', 0x4C),
            ('l', 0x4C),
            ('o', 0x4F),
        ];
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
            assert_eq!(tp.text, "[UNOBSERVED_TEXT]");
            assert_eq!(tp.character_count, 2);
            assert!(tp.is_redacted);
        }
    }

    #[test]
    fn test_mouse_move_aggregator_continuous() {
        let mut agg = MouseMoveAggregator::new(Duration::from_millis(1000));
        let ts = DualTimestamp::now();

        // Feed 50 continuous mouse moves from (100, 100) to (200, 200)
        for i in 0..50 {
            let raw = RawEvent::new(
                i + 1,
                GlobalEventId::new(500 + i),
                ts,
                "mach1".to_string(),
                1,
                "user1".to_string(),
                EventSource::Win32Hook,
                i + 1,
                RawEventPayload::Mouse(RawMouseEvent {
                    event_type: "MOUSE_MOVE".to_string(),
                    button: MouseButton::None,
                    physical_x: 100 + i as i32 * 2,
                    physical_y: 100 + i as i32 * 2,
                    normalized_x: 0.1,
                    normalized_y: 0.1,
                    delta_x: 0.0,
                    delta_y: 0.0,
                    monitor_id: 1,
                    coords: Point2D::new(100 + i as i32 * 2, 100 + i as i32 * 2, 0.1, 0.1),
                    state: String::new(),
                }),
            );
            let flushed = agg.on_event(&raw);
            assert!(flushed.is_none(), "Continuous moves should be buffered");
        }

        // Flush active stream
        let flushed = agg.flush().expect("Expected trajectory to be flushed");
        if let RawEventPayload::Mouse(ref m) = flushed.payload {
            assert_eq!(m.event_type, "MOUSE_MOVE");
            assert_eq!(m.physical_x, 198); // 100 + 49 * 2
            assert_eq!(m.physical_y, 198);
            assert_eq!(m.delta_x, 98.0);
            assert_eq!(m.delta_y, 98.0);
            assert!(m.state.contains("start=(100, 100)"));
            assert!(m.state.contains("end=(198, 198)"));
            assert!(m.state.contains("samples=50"));
        } else {
            panic!("Expected Mouse payload");
        }
    }

    #[test]
    fn test_mouse_move_aggregator_interrupted_by_click() {
        let mut agg = MouseMoveAggregator::new(Duration::from_millis(1000));
        let ts = DualTimestamp::now();

        // 1. Move to (150, 150)
        let move_ev = RawEvent::new(
            1,
            GlobalEventId::new(600),
            ts,
            "mach1".to_string(),
            1,
            "user1".to_string(),
            EventSource::Win32Hook,
            1,
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
                state: String::new(),
            }),
        );
        agg.on_event(&move_ev);

        // 2. Click arrives -> interrupts mouse move stream immediately
        let click_ev = RawEvent::new(
            2,
            GlobalEventId::new(601),
            ts,
            "mach1".to_string(),
            1,
            "user1".to_string(),
            EventSource::Win32Hook,
            2,
            RawEventPayload::Mouse(RawMouseEvent {
                event_type: "CLICK".to_string(),
                button: MouseButton::Left,
                physical_x: 150,
                physical_y: 150,
                normalized_x: 0.15,
                normalized_y: 0.15,
                delta_x: 0.0,
                delta_y: 0.0,
                monitor_id: 1,
                coords: Point2D::new(150, 150, 0.15, 0.15),
                state: "PRESSED".to_string(),
            }),
        );
        let flushed = agg
            .on_event(&click_ev)
            .expect("Interruption by click should flush trajectory");
        if let RawEventPayload::Mouse(ref m) = flushed.payload {
            assert_eq!(m.event_type, "MOUSE_MOVE");
            assert_eq!(m.physical_x, 150);
            assert_eq!(m.physical_y, 150);
            assert!(m.state.contains("samples=1"));
        } else {
            panic!("Expected Mouse payload");
        }
    }

    #[test]
    fn test_mouse_move_aggregator_timeout() {
        let mut agg = MouseMoveAggregator::new(Duration::from_millis(50));
        let ts = DualTimestamp::now();

        let move_ev = RawEvent::new(
            1,
            GlobalEventId::new(700),
            ts,
            "mach1".to_string(),
            1,
            "user1".to_string(),
            EventSource::Win32Hook,
            1,
            RawEventPayload::Mouse(RawMouseEvent {
                event_type: "MOUSE_MOVE".to_string(),
                button: MouseButton::None,
                physical_x: 300,
                physical_y: 300,
                normalized_x: 0.3,
                normalized_y: 0.3,
                delta_x: 0.0,
                delta_y: 0.0,
                monitor_id: 1,
                coords: Point2D::new(300, 300, 0.3, 0.3),
                state: String::new(),
            }),
        );
        agg.on_event(&move_ev);
        assert!(agg.check_timeout().is_none());

        sleep(Duration::from_millis(60));
        let flushed = agg.check_timeout().expect("Should flush on timeout");
        if let RawEventPayload::Mouse(ref m) = flushed.payload {
            assert_eq!(m.physical_x, 300);
        } else {
            panic!("Expected Mouse payload");
        }
    }

    #[test]
    fn test_raw_event_aggregator_typing_burst() {
        let mut agg = RawEventAggregator::new(Duration::from_millis(1000), Duration::from_millis(1000));
        let ts = DualTimestamp::now();

        let keys = ['A', 'B', 'C', 'D'];
        for (i, &ch) in keys.iter().enumerate() {
            let ev = RawEvent::new(
                i as u64 + 1,
                GlobalEventId::new(800 + i as u64),
                ts,
                "mach1".to_string(),
                1,
                "user1".to_string(),
                EventSource::Win32Hook,
                i as u64 + 1,
                RawEventPayload::Keyboard(RawKeyboardEvent {
                    event_type: "KEY_DOWN".to_string(),
                    vk_code: ch as u32,
                    scan_code: 0,
                    key_name: ch.to_string(),
                    modifiers: Default::default(),
                    is_injected: false,
                }),
            );
            let out = agg.process_raw_event(ev);
            assert!(out.is_empty(), "Typing keystrokes should be buffered");
        }

        // Enter key interrupts typing burst
        let enter_ev = RawEvent::new(
            10,
            GlobalEventId::new(810),
            ts,
            "mach1".to_string(),
            1,
            "user1".to_string(),
            EventSource::Win32Hook,
            10,
            RawEventPayload::Keyboard(RawKeyboardEvent {
                event_type: "KEY_DOWN".to_string(),
                vk_code: 0x0D, // VK_RETURN
                scan_code: 0,
                key_name: "ENTER".to_string(),
                modifiers: Default::default(),
                is_injected: false,
            }),
        );
        let out = agg.process_raw_event(enter_ev);
        assert_eq!(out.len(), 2, "Expected flushed typing burst followed by ENTER key");
        if let RawEventPayload::Keyboard(ref kb) = out[0].payload {
            assert!(kb.key_name.contains("[UNOBSERVED_TEXT]"));
            assert!(kb.key_name.contains("chars=4"));
        } else {
            panic!("Expected Keyboard payload for burst");
        }
        if let RawEventPayload::Keyboard(ref kb) = out[1].payload {
            assert_eq!(kb.key_name, "ENTER");
        } else {
            panic!("Expected Keyboard payload for enter");
        }
    }
}
