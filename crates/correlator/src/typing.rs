use core_types::action::{ActionParameters, ActionType, CanonicalAction, TypeTextParams};
use core_types::id::{GlobalEventId, SessionId};
use core_types::metadata::{ContextMetadata, TargetMetadata};
use core_types::timestamp::DualTimestamp;
use std::time::{Duration, Instant};

/// Aggregates individual keystrokes into debounced typing bursts (`TYPE_TEXT`).
pub struct TypingBurstAggregator {
    debounce_duration: Duration,
    current_burst: Option<ActiveTypingBurst>,
}

struct ActiveTypingBurst {
    start_time: DualTimestamp,
    last_keystroke_instant: Instant,
    buffer: String,
    char_count: usize,
    backspace_count: usize,
    enter_pressed: bool,
    target: TargetMetadata,
    context: ContextMetadata,
}

impl Default for TypingBurstAggregator {
    fn default() -> Self {
        Self::new(Duration::from_millis(500))
    }
}

impl TypingBurstAggregator {
    pub fn new(debounce_duration: Duration) -> Self {
        Self {
            debounce_duration,
            current_burst: None,
        }
    }

    /// Process a new keystroke event. If the burst has expired, returns the completed `CanonicalAction`.
    pub fn on_keystroke(
        &mut self,
        timestamp: DualTimestamp,
        vk_code: u32,
        key_name: &str,
        is_down: bool,
        target: TargetMetadata,
        context: ContextMetadata,
        session_id: &SessionId,
        next_global_id: u64,
        session_event_id: u64,
    ) -> Option<CanonicalAction> {
        if !is_down {
            return None;
        }

        let now = Instant::now();
        let mut completed_action = None;

        // Check if existing burst has timed out or target changed
        if let Some(ref active) = self.current_burst {
            if now.duration_since(active.last_keystroke_instant) > self.debounce_duration
                || active.target.automation_id != target.automation_id
            {
                completed_action = self.flush(session_id, next_global_id, session_event_id);
            }
        }

        // Add to active burst or start new burst
        let burst = self.current_burst.get_or_insert_with(|| ActiveTypingBurst {
            start_time: timestamp,
            last_keystroke_instant: now,
            buffer: String::new(),
            char_count: 0,
            backspace_count: 0,
            enter_pressed: false,
            target,
            context,
        });

        burst.last_keystroke_instant = now;

        match vk_code {
            0x08 => {
                // Backspace
                burst.backspace_count += 1;
                burst.buffer.pop();
            }
            0x0D => {
                // Enter
                burst.enter_pressed = true;
                burst.char_count += 1;
            }
            0x20 => {
                // Space
                burst.char_count += 1;
                burst.buffer.push(' ');
            }
            _ => {
                burst.char_count += 1;
                if key_name.len() == 1 {
                    burst.buffer.push_str(key_name);
                } else if !key_name.starts_with("VK_") {
                    burst.buffer.push_str(key_name);
                }
            }
        }

        completed_action
    }

    /// Check if current burst has timed out and should be flushed.
    pub fn check_timeout(
        &mut self,
        session_id: &SessionId,
        next_global_id: u64,
        session_event_id: u64,
    ) -> Option<CanonicalAction> {
        if let Some(ref active) = self.current_burst {
            if Instant::now().duration_since(active.last_keystroke_instant) > self.debounce_duration
            {
                return self.flush(session_id, next_global_id, session_event_id);
            }
        }
        None
    }

    /// Flush active burst immediately.
    pub fn flush(
        &mut self,
        session_id: &SessionId,
        next_global_id: u64,
        session_event_id: u64,
    ) -> Option<CanonicalAction> {
        let active = self.current_burst.take()?;
        if active.char_count == 0 && active.backspace_count == 0 {
            return None;
        }

        let is_pwd = active.target.is_password;
        let text_param = TypeTextParams {
            text: if is_pwd {
                "[PASSWORD_REDACTED]".to_string()
            } else {
                active.buffer.clone()
            },
            length: active.char_count,
            is_redacted: is_pwd,
            character_count: active.char_count,
            backspace_count: active.backspace_count,
            enter_pressed: active.enter_pressed,
        };

        Some(CanonicalAction {
            schema: core_types::SCHEMA_IDENTIFIER.to_string(),
            schema_version: core_types::SCHEMA_VERSION.to_string(),
            global_event_id: GlobalEventId::new(next_global_id),
            session_id: session_id.clone(),
            session_event_id,
            timestamp: active.start_time,
            action_type: ActionType::TypeText,
            confidence: 1.0,
            target: active.target,
            context: active.context,
            before: Default::default(),
            parameters: ActionParameters::TypeText(text_param),
            after: Default::default(),
            evidence: Default::default(),
            state_evidence: None,
            duration_ms: Some(
                Instant::now()
                    .duration_since(active.last_keystroke_instant)
                    .as_millis() as u64,
            ),
        })
    }
}
