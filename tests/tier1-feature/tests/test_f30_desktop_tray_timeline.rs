use correlator::CorrelationEngine;
use core_types::action::ActionType;
use core_types::event::{EventSource, RawEvent, RawEventPayload, RawMouseEvent};
use core_types::id::GlobalEventId;
use core_types::metadata::{MouseButton, TargetMetadata};
use core_types::timestamp::DualTimestamp;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

#[test]
fn test_f30_timeline_action_flow() {
    let global_seq = Arc::new(AtomicU64::new(1));
    let mut engine = CorrelationEngine::new("WS01_20260829_040000_a1b2c3d4", "alice", "WS01", global_seq);

    let raw = RawEvent::new(
        1,
        GlobalEventId::new(1),
        DualTimestamp::now(),
        "WS01".to_string(),
        1,
        "alice".to_string(),
        EventSource::Win32Hook,
        1,
        RawEventPayload::Mouse(RawMouseEvent {
            event_type: "CLICK".to_string(),
            button: MouseButton::Left,
            physical_x: 350,
            physical_y: 220,
            normalized_x: 0.18,
            normalized_y: 0.20,
            delta_x: 0.0,
            delta_y: 0.0,
            monitor_id: 1,
            ..Default::default()
        }),
    );

    let target = TargetMetadata {
        name: Some("Submit".to_string()),
        control_type: Some("Button".to_string()),
        automation_id: Some("btn_submit".to_string()),
        ..Default::default()
    };

    let actions = engine.process_event(&raw, Some(target));
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].action_type, ActionType::Click);
    assert_eq!(actions[0].target.automation_id.as_deref(), Some("btn_submit"));
    assert_eq!(actions[0].confidence, 1.0);
}
