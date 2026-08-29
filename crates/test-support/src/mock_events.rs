use core_types::{
    ActionType, BoundingBox, CanonicalAction, DualTimestamp, EventSource, GlobalEventId,
    MouseButton, Point2D, RawEvent, RawEventPayload, RawMouseEvent, SCHEMA_IDENTIFIER,
    SCHEMA_VERSION, SessionId, TargetMetadata,
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

pub struct MockEventGenerator {
    rng: ChaCha8Rng,
    current_global_id: u64,
    current_session_event_id: u64,
    session_id: String,
    machine_id: String,
    monotonic_clock_ns: u64,
}

impl MockEventGenerator {
    pub fn new(seed: u64, session_id: &str, machine_id: &str) -> Self {
        Self {
            rng: ChaCha8Rng::seed_from_u64(seed),
            current_global_id: 1,
            current_session_event_id: 1,
            session_id: session_id.to_string(),
            machine_id: machine_id.to_string(),
            monotonic_clock_ns: 1_000_000_000,
        }
    }

    pub fn next_timestamp(&mut self, delta_ms: u64) -> DualTimestamp {
        self.monotonic_clock_ns += delta_ms * 1_000_000;
        DualTimestamp {
            wall_time_utc: chrono::Utc::now(),
            monotonic_ns: self.monotonic_clock_ns,
            timezone_offset_secs: 7 * 3600,
        }
    }

    pub fn generate_mouse_click_raw(&mut self, x: i32, y: i32, button: MouseButton) -> RawEvent {
        let event_id = self.current_global_id;
        self.current_global_id += 1;
        let ts = self.next_timestamp(50);

        RawEvent {
            schema: SCHEMA_IDENTIFIER.to_string(),
            schema_version: SCHEMA_VERSION.to_string(),
            event_id,
            global_event_id: Some(GlobalEventId::new(event_id)),
            timestamp: ts,
            machine_id: self.machine_id.clone(),
            windows_session_id: 1,
            user_id: "test-user".to_string(),
            source: EventSource::InputHook,
            source_sequence: event_id,
            payload: RawEventPayload::Mouse(RawMouseEvent {
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
            }),
        }
    }

    pub fn generate_canonical_action(
        &mut self,
        action_type: ActionType,
        target_name: &str,
    ) -> CanonicalAction {
        let global_id = self.current_global_id;
        let session_event_id = self.current_session_event_id;
        self.current_global_id += 1;
        self.current_session_event_id += 1;
        let ts = self.next_timestamp(100);

        CanonicalAction {
            schema: SCHEMA_IDENTIFIER.to_string(),
            schema_version: SCHEMA_VERSION.to_string(),
            global_event_id: GlobalEventId(global_id),
            session_id: SessionId::new(self.session_id.clone()),
            session_event_id,
            timestamp: ts,
            action_type,
            confidence: 0.95,
            target: TargetMetadata {
                name: Some(target_name.to_string()),
                control_type: Some("Button".to_string()),
                automation_id: Some(format!("btn_{target_name}")),
                class_name: Some("WpfButton".to_string()),
                framework_id: Some("WPF".to_string()),
                bounding_box: Some(BoundingBox {
                    x: 100,
                    y: 200,
                    width: 80,
                    height: 30,
                }),
                bounding_rect: Some(
                    BoundingBox {
                        x: 100,
                        y: 200,
                        width: 80,
                        height: 30,
                    }
                    .to_bounding_rect(),
                ),
                is_password: false,
                is_enabled: Some(true),
                is_keyboard_focusable: Some(true),
                value: None,
                help_text: None,
                ancestor_chain: vec![],
                ancestors: vec![],
                dom_selector: None,
                xpath: None,
            },
            context: core_types::ContextMetadata {
                application: core_types::ApplicationContext {
                    process_name: "testapp.exe".to_string(),
                    pid: 4200,
                    executable_path: Some("C:\\Program Files\\TestApp\\testapp.exe".to_string()),
                    app_id: None,
                    is_elevated: false,
                },
                window: core_types::WindowContext {
                    hwnd: 0x1004A,
                    title: "Test Application".to_string(),
                    bounds: BoundingBox {
                        x: 0,
                        y: 0,
                        width: 1024,
                        height: 768,
                    }
                    .to_bounding_rect(),
                    is_maximized: false,
                    is_minimized: false,
                    is_foreground: true,
                    is_fullscreen: false,
                    dpi: 96,
                },
                browser: None,
                display: core_types::DisplayContext::default(),
                user_id: "test-user".to_string(),
                machine_id: self.machine_id.clone(),
                process_name: "testapp.exe".to_string(),
                process_id: 4200,
                executable_path: "C:\\Program Files\\TestApp\\testapp.exe".to_string(),
                window_title: "Test Application".to_string(),
                window_handle: 0x1004A,
                monitor_id: 0,
                is_fullscreen: false,
                is_elevated: false,
            },
            before: core_types::StateSnapshot::default(),
            parameters: core_types::ActionParameters::None,
            after: core_types::StateSnapshot::default(),
            evidence: core_types::ActionEvidence::default(),
            state_evidence: None,
            duration_ms: Some(25),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_event_generator_deterministic_seed() {
        let mut gen1 = MockEventGenerator::new(0xDEADBEEF, "sess_1", "mach_1");
        let mut gen2 = MockEventGenerator::new(0xDEADBEEF, "sess_1", "mach_1");

        let e1 = gen1.generate_mouse_click_raw(100, 200, MouseButton::Left);
        let e2 = gen2.generate_mouse_click_raw(100, 200, MouseButton::Left);

        assert_eq!(e1.event_id, e2.event_id);
        assert_eq!(e1.timestamp.monotonic_ns, e2.timestamp.monotonic_ns);
    }
}
