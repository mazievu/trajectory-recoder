use core_types::action::{ActionParameters, ActionType, CanonicalAction, ScrollParams};
use core_types::id::{GlobalEventId, SessionId};
use core_types::metadata::{ContextMetadata, ScrollDirection, TargetMetadata};
use core_types::timestamp::DualTimestamp;
use std::time::{Duration, Instant};

/// Aggregates individual mouse wheel ticks into debounced scroll bursts (`SCROLL`).
pub struct ScrollBurstAggregator {
    debounce_duration: Duration,
    current_burst: Option<ActiveScrollBurst>,
}

struct ActiveScrollBurst {
    start_time: DualTimestamp,
    last_tick_instant: Instant,
    total_delta_x: f64,
    total_delta_y: f64,
    wheel_ticks: u32,
    target: TargetMetadata,
    context: ContextMetadata,
}

impl Default for ScrollBurstAggregator {
    fn default() -> Self {
        Self::new(Duration::from_millis(300))
    }
}

impl ScrollBurstAggregator {
    pub fn new(debounce_duration: Duration) -> Self {
        Self {
            debounce_duration,
            current_burst: None,
        }
    }

    pub fn on_wheel(
        &mut self,
        timestamp: DualTimestamp,
        delta_x: f64,
        delta_y: f64,
        target: TargetMetadata,
        context: ContextMetadata,
        session_id: &SessionId,
        next_global_id: u64,
        session_event_id: u64,
    ) -> Option<CanonicalAction> {
        let now = Instant::now();
        let mut completed_action = None;

        if let Some(ref active) = self.current_burst {
            if now.duration_since(active.last_tick_instant) > self.debounce_duration
                || active.target.automation_id != target.automation_id
            {
                completed_action = self.flush(session_id, next_global_id, session_event_id);
            }
        }

        let burst = self.current_burst.get_or_insert_with(|| ActiveScrollBurst {
            start_time: timestamp,
            last_tick_instant: now,
            total_delta_x: 0.0,
            total_delta_y: 0.0,
            wheel_ticks: 0,
            target,
            context,
        });

        burst.last_tick_instant = now;
        burst.total_delta_x += delta_x;
        burst.total_delta_y += delta_y;
        burst.wheel_ticks += 1;

        completed_action
    }

    pub fn check_timeout(
        &mut self,
        session_id: &SessionId,
        next_global_id: u64,
        session_event_id: u64,
    ) -> Option<CanonicalAction> {
        if let Some(ref active) = self.current_burst {
            if Instant::now().duration_since(active.last_tick_instant) > self.debounce_duration {
                return self.flush(session_id, next_global_id, session_event_id);
            }
        }
        None
    }

    pub fn flush(
        &mut self,
        session_id: &SessionId,
        next_global_id: u64,
        session_event_id: u64,
    ) -> Option<CanonicalAction> {
        let active = self.current_burst.take()?;
        if active.wheel_ticks == 0 {
            return None;
        }

        let direction = if active.total_delta_y.abs() >= active.total_delta_x.abs() {
            if active.total_delta_y < 0.0 {
                ScrollDirection::VerticalDown
            } else {
                ScrollDirection::VerticalUp
            }
        } else {
            if active.total_delta_x < 0.0 {
                ScrollDirection::HorizontalRight
            } else {
                ScrollDirection::HorizontalLeft
            }
        };

        let scroll_params = ScrollParams {
            delta_x: active.total_delta_x,
            delta_y: active.total_delta_y,
            direction,
            container_type: active.target.control_type.clone(),
            wheel_ticks: active.wheel_ticks,
        };

        Some(CanonicalAction {
            schema: core_types::SCHEMA_IDENTIFIER.to_string(),
            schema_version: core_types::SCHEMA_VERSION.to_string(),
            global_event_id: GlobalEventId::new(next_global_id),
            session_id: session_id.clone(),
            session_event_id,
            timestamp: active.start_time,
            action_type: ActionType::Scroll,
            confidence: 1.0,
            target: active.target,
            context: active.context,
            before: Default::default(),
            parameters: ActionParameters::Scroll(scroll_params),
            after: Default::default(),
            evidence: Default::default(),
            state_evidence: None,
            duration_ms: Some(
                Instant::now()
                    .duration_since(active.last_tick_instant)
                    .as_millis() as u64,
            ),
        })
    }
}
