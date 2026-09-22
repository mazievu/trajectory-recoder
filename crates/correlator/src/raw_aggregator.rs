use crate::mouse_move::MouseMoveAggregator;
use crate::typing::{is_modifier_key, is_text_input_key};
use core_types::event::{RawEvent, RawEventPayload, RawKeyboardEvent};
use core_types::timestamp::DualTimestamp;
use std::time::{Duration, Instant};

/// Unified client-side raw event aggregator.
///
/// Merges continuous, uninterrupted mouse movements (gap <= 1.0s) and continuous
/// keyboard text typing bursts (gap <= 1.0s) before persistence to raw storage,
/// eliminating redundant raw telemetry while preserving full trajectory metrics.
pub struct RawEventAggregator {
    mouse_aggregator: MouseMoveAggregator,
    keyboard_timeout: Duration,
    active_keyboard_burst: Option<ActiveRawTypingBurst>,
}

struct ActiveRawTypingBurst {
    start_time: DualTimestamp,
    start_instant: Instant,
    last_instant: Instant,
    char_count: usize,
    backspace_count: usize,
    template_event: RawEvent,
}

impl Default for RawEventAggregator {
    fn default() -> Self {
        Self::new(Duration::from_millis(1000), Duration::from_millis(1000))
    }
}

impl RawEventAggregator {
    pub fn new(mouse_timeout: Duration, keyboard_timeout: Duration) -> Self {
        Self {
            mouse_aggregator: MouseMoveAggregator::new(mouse_timeout),
            keyboard_timeout,
            active_keyboard_burst: None,
        }
    }

    /// Process a raw event through the aggregation pipelines.
    ///
    /// Returns any completed aggregated events or passthrough events in chronological order.
    pub fn process_raw_event(&mut self, event: RawEvent) -> Vec<RawEvent> {
        let mut out = Vec::new();

        match &event.payload {
            RawEventPayload::Mouse(m) if m.event_type == "MOUSE_MOVE" => {
                // Moving mouse interrupts any pending typing burst
                if let Some(flushed_kb) = self.flush_keyboard() {
                    out.push(flushed_kb);
                }
                if let Some(flushed_mouse) = self.mouse_aggregator.on_event(&event) {
                    out.push(flushed_mouse);
                }
            }
            RawEventPayload::Mouse(_) => {
                // Non-move mouse event (click, down, up, wheel) interrupts both streams
                if let Some(flushed_mouse) = self.mouse_aggregator.flush() {
                    out.push(flushed_mouse);
                }
                if let Some(flushed_kb) = self.flush_keyboard() {
                    out.push(flushed_kb);
                }
                out.push(event);
            }
            RawEventPayload::Keyboard(kb) => {
                // Keyboard interaction interrupts mouse movement
                if let Some(flushed_mouse) = self.mouse_aggregator.flush() {
                    out.push(flushed_mouse);
                }

                // Shortcuts (Ctrl or Alt modifier) flush typing and pass through
                if kb.modifiers.ctrl || kb.modifiers.alt {
                    if let Some(flushed_kb) = self.flush_keyboard() {
                        out.push(flushed_kb);
                    }
                    out.push(event);
                    return out;
                }

                if is_text_input_key(kb.vk_code) {
                    if kb.event_type == "KEY_DOWN" {
                        let now = Instant::now();
                        if let Some(ref active) = self.active_keyboard_burst {
                            if now.duration_since(active.last_instant) > self.keyboard_timeout {
                                if let Some(flushed) = self.flush_keyboard() {
                                    out.push(flushed);
                                }
                            }
                        }

                        if let Some(ref mut burst) = self.active_keyboard_burst {
                            burst.char_count += 1;
                            burst.last_instant = now;
                        } else {
                            self.active_keyboard_burst = Some(ActiveRawTypingBurst {
                                start_time: event.timestamp,
                                start_instant: now,
                                last_instant: now,
                                char_count: 1,
                                backspace_count: 0,
                                template_event: event,
                            });
                        }
                    }
                    // KEY_UP for text input keys is absorbed into the burst
                } else if kb.vk_code == 0x08 {
                    // Backspace
                    if kb.event_type == "KEY_DOWN" {
                        let now = Instant::now();
                        if let Some(ref active) = self.active_keyboard_burst {
                            if now.duration_since(active.last_instant) > self.keyboard_timeout {
                                if let Some(flushed) = self.flush_keyboard() {
                                    out.push(flushed);
                                }
                            }
                        }

                        if let Some(ref mut burst) = self.active_keyboard_burst {
                            burst.backspace_count += 1;
                            burst.last_instant = now;
                        } else {
                            self.active_keyboard_burst = Some(ActiveRawTypingBurst {
                                start_time: event.timestamp,
                                start_instant: now,
                                last_instant: now,
                                char_count: 0,
                                backspace_count: 1,
                                template_event: event,
                            });
                        }
                    }
                } else if is_modifier_key(kb.vk_code) {
                    // Standalone Shift/Caps lock modifies input but does not break typing
                } else {
                    // Enter, Escape, Tab, navigation, function keys flush typing and pass through
                    if let Some(flushed_kb) = self.flush_keyboard() {
                        out.push(flushed_kb);
                    }
                    out.push(event);
                }
            }
            _ => {
                // Window, UIA, Browser, Clipboard, File, etc. interrupt all streams
                if let Some(flushed_mouse) = self.mouse_aggregator.flush() {
                    out.push(flushed_mouse);
                }
                if let Some(flushed_kb) = self.flush_keyboard() {
                    out.push(flushed_kb);
                }
                out.push(event);
            }
        }

        out
    }

    /// Check for timeouts on active mouse and keyboard streams.
    pub fn check_timeout(&mut self) -> Vec<RawEvent> {
        let mut out = Vec::new();
        if let Some(flushed_mouse) = self.mouse_aggregator.check_timeout() {
            out.push(flushed_mouse);
        }
        if let Some(ref active) = self.active_keyboard_burst {
            if active.last_instant.elapsed() > self.keyboard_timeout {
                if let Some(flushed_kb) = self.flush_keyboard() {
                    out.push(flushed_kb);
                }
            }
        }
        out
    }

    /// Flush all buffered streams immediately.
    pub fn flush_all(&mut self) -> Vec<RawEvent> {
        let mut out = Vec::new();
        if let Some(flushed_mouse) = self.mouse_aggregator.flush() {
            out.push(flushed_mouse);
        }
        if let Some(flushed_kb) = self.flush_keyboard() {
            out.push(flushed_kb);
        }
        out
    }

    fn flush_keyboard(&mut self) -> Option<RawEvent> {
        let burst = self.active_keyboard_burst.take()?;
        if burst.char_count == 0 && burst.backspace_count == 0 {
            return None;
        }

        let duration_ms = burst
            .last_instant
            .duration_since(burst.start_instant)
            .as_millis() as u64;

        let mut event = burst.template_event;
        event.timestamp = burst.start_time;
        event.payload = RawEventPayload::Keyboard(RawKeyboardEvent {
            event_type: "KEY_DOWN".to_string(),
            vk_code: 0,
            scan_code: 0,
            key_name: format!(
                "[UNOBSERVED_TEXT] (chars={}, backspaces={}, duration={}ms)",
                burst.char_count, burst.backspace_count, duration_ms
            ),
            modifiers: Default::default(),
            is_injected: false,
        });

        Some(event)
    }
}
