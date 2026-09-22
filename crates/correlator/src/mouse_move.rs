use core_types::event::{RawEvent, RawEventPayload, RawMouseEvent};
use core_types::metadata::{MouseButton, Point2D};
use core_types::timestamp::DualTimestamp;
use std::time::{Duration, Instant};

/// Aggregates high-frequency continuous mouse movements into single trajectory records.
///
/// Continuous mouse moves occurring within `timeout` (default: 1.0s) without intervening
/// events are merged from the initial starting coordinate to the final destination.
pub struct MouseMoveAggregator {
    timeout: Duration,
    current_stream: Option<ActiveMouseStream>,
}

struct ActiveMouseStream {
    start_time: DualTimestamp,
    start_instant: Instant,
    last_instant: Instant,
    start_x: i32,
    start_y: i32,
    prev_x: i32,
    prev_y: i32,
    last_x: i32,
    last_y: i32,
    last_norm_x: f32,
    last_norm_y: f32,
    monitor_id: u32,
    sample_count: u32,
    total_distance: f64,
    template_event: RawEvent,
}

impl Default for MouseMoveAggregator {
    fn default() -> Self {
        Self::new(Duration::from_millis(1000))
    }
}

impl MouseMoveAggregator {
    pub fn new(timeout: Duration) -> Self {
        Self {
            timeout,
            current_stream: None,
        }
    }

    /// Process an incoming RawEvent.
    ///
    /// - If the event is a `MOUSE_MOVE`, it is aggregated into the active stream.
    ///   Returns `Some(flushed_previous)` only if an existing stream timed out.
    /// - If the event is NOT a `MOUSE_MOVE`, any active stream is immediately flushed
    ///   and returned so the caller can persist it prior to the interrupting event.
    pub fn on_event(&mut self, event: &RawEvent) -> Option<RawEvent> {
        match &event.payload {
            RawEventPayload::Mouse(m) if m.event_type == "MOUSE_MOVE" => {
                let now = Instant::now();
                let mut flushed = None;

                if let Some(ref active) = self.current_stream {
                    if now.duration_since(active.last_instant) > self.timeout {
                        flushed = self.flush();
                    }
                }

                if let Some(ref mut active) = self.current_stream {
                    let dx = (m.physical_x - active.prev_x) as f64;
                    let dy = (m.physical_y - active.prev_y) as f64;
                    active.total_distance += (dx * dx + dy * dy).sqrt();
                    active.prev_x = m.physical_x;
                    active.prev_y = m.physical_y;
                    active.last_x = m.physical_x;
                    active.last_y = m.physical_y;
                    active.last_norm_x = m.normalized_x;
                    active.last_norm_y = m.normalized_y;
                    active.sample_count += 1;
                    active.last_instant = now;
                } else {
                    self.current_stream = Some(ActiveMouseStream {
                        start_time: event.timestamp,
                        start_instant: now,
                        last_instant: now,
                        start_x: m.physical_x,
                        start_y: m.physical_y,
                        prev_x: m.physical_x,
                        prev_y: m.physical_y,
                        last_x: m.physical_x,
                        last_y: m.physical_y,
                        last_norm_x: m.normalized_x,
                        last_norm_y: m.normalized_y,
                        monitor_id: m.monitor_id,
                        sample_count: 1,
                        total_distance: 0.0,
                        template_event: event.clone(),
                    });
                }

                flushed
            }
            _ => {
                // Interruption: flush active mouse stream
                self.flush()
            }
        }
    }

    /// Check if the active stream has timed out due to inactivity (> timeout).
    pub fn check_timeout(&mut self) -> Option<RawEvent> {
        if let Some(ref active) = self.current_stream {
            if active.last_instant.elapsed() > self.timeout {
                return self.flush();
            }
        }
        None
    }

    /// Flush any active mouse move stream immediately.
    pub fn flush(&mut self) -> Option<RawEvent> {
        let stream = self.current_stream.take()?;
        let duration_ms = stream
            .last_instant
            .duration_since(stream.start_instant)
            .as_millis() as u64;

        let delta_x = (stream.last_x - stream.start_x) as f64;
        let delta_y = (stream.last_y - stream.start_y) as f64;

        let state = format!(
            "trajectory: start=({}, {}), end=({}, {}), dist={:.1}px, samples={}, duration={}ms",
            stream.start_x,
            stream.start_y,
            stream.last_x,
            stream.last_y,
            stream.total_distance,
            stream.sample_count,
            duration_ms
        );

        let mut event = stream.template_event;
        event.timestamp = stream.start_time;
        event.payload = RawEventPayload::Mouse(RawMouseEvent {
            event_type: "MOUSE_MOVE".to_string(),
            button: MouseButton::None,
            coords: Point2D::new(
                stream.last_x,
                stream.last_y,
                stream.last_norm_x,
                stream.last_norm_y,
            ),
            monitor_id: stream.monitor_id,
            delta_x,
            delta_y,
            state,
            physical_x: stream.last_x,
            physical_y: stream.last_y,
            normalized_x: stream.last_norm_x,
            normalized_y: stream.last_norm_y,
        });

        Some(event)
    }
}
