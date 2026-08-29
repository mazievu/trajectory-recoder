use core_types::action::{ActionParameters, ActionType, CanonicalAction, DragDropParams};
use core_types::id::{GlobalEventId, SessionId};
use core_types::metadata::{ContextMetadata, MouseButton, Point2D, TargetMetadata};
use core_types::timestamp::DualTimestamp;
use std::time::Instant;

/// State machine tracking mouse interactions to distinguish standard clicks from drag-and-drop operations.
pub struct DragDropStateMachine {
    distance_threshold_px: f64,
    active_drag: Option<ActiveDragState>,
}

struct ActiveDragState {
    button: MouseButton,
    start_timestamp: DualTimestamp,
    start_instant: Instant,
    start_coords: Point2D,
    last_coords: Point2D,
    is_dragging: bool,
    source_target: TargetMetadata,
    context: ContextMetadata,
}

impl Default for DragDropStateMachine {
    fn default() -> Self {
        Self::new(5.0) // 5px threshold
    }
}

impl DragDropStateMachine {
    pub fn new(distance_threshold_px: f64) -> Self {
        Self {
            distance_threshold_px,
            active_drag: None,
        }
    }

    /// On mouse down at point
    pub fn on_mouse_down(
        &mut self,
        timestamp: DualTimestamp,
        button: MouseButton,
        coords: Point2D,
        target: TargetMetadata,
        context: ContextMetadata,
    ) {
        if button == MouseButton::Left || button == MouseButton::Right {
            self.active_drag = Some(ActiveDragState {
                button,
                start_timestamp: timestamp,
                start_instant: Instant::now(),
                start_coords: coords,
                last_coords: coords,
                is_dragging: false,
                source_target: target,
                context,
            });
        }
    }

    /// On mouse move: updates state and tests distance threshold
    pub fn on_mouse_move(&mut self, coords: Point2D) {
        if let Some(ref mut state) = self.active_drag {
            state.last_coords = coords;
            let dx = (coords.physical_x - state.start_coords.physical_x) as f64;
            let dy = (coords.physical_y - state.start_coords.physical_y) as f64;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist >= self.distance_threshold_px {
                state.is_dragging = true;
            }
        }
    }

    /// Whether a release for `button` can complete the current pointer gesture.
    ///
    /// Callers use this before `on_mouse_up` to distinguish an ordinary click
    /// from an unmatched mouse-up event.
    pub fn is_active_for(&self, button: MouseButton) -> bool {
        self.active_drag
            .as_ref()
            .is_some_and(|state| state.button == button)
    }

    /// On mouse up: returns `Some(CanonicalAction)` if it was a drag & drop, or `None` if standard click
    pub fn on_mouse_up(
        &mut self,
        timestamp: DualTimestamp,
        button: MouseButton,
        coords: Point2D,
        dest_target: TargetMetadata,
        session_id: &SessionId,
        next_global_id: u64,
        session_event_id: u64,
    ) -> Option<CanonicalAction> {
        let state = self.active_drag.take()?;
        if state.button != button || !state.is_dragging {
            return None;
        }

        let dx = (coords.physical_x - state.start_coords.physical_x) as f64;
        let dy = (coords.physical_y - state.start_coords.physical_y) as f64;
        let distance_px = (dx * dx + dy * dy).sqrt();
        let duration_ms = Instant::now().duration_since(state.start_instant).as_millis() as u64;

        let params = DragDropParams {
            start_coords: state.start_coords,
            end_coords: coords,
            distance_px,
            path_summary: Some(format!("dx={:.1}, dy={:.1}", dx, dy)),
            source_target: state.source_target.automation_id.or(state.source_target.name),
            destination_target: dest_target.automation_id.clone().or(dest_target.name.clone()),
        };

        Some(CanonicalAction {
            schema: core_types::SCHEMA_IDENTIFIER.to_string(),
            schema_version: core_types::SCHEMA_VERSION.to_string(),
            global_event_id: GlobalEventId::new(next_global_id),
            session_id: session_id.clone(),
            session_event_id,
            timestamp: state.start_timestamp,
            action_type: ActionType::DragDrop,
            confidence: 0.95,
            target: dest_target,
            context: state.context,
            before: Default::default(),
            parameters: ActionParameters::DragDrop(params),
            after: Default::default(),
            evidence: Default::default(),
            state_evidence: None,
            duration_ms: Some(duration_ms),
        })
    }
}
