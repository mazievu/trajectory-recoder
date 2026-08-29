use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DragState {
    Idle,
    Dragging,
    DroppedSuccess,
    DroppedCancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DragDropZone {
    pub source_id: String,
    pub target_id: String,
    pub current_state: DragState,
    pub payload_data: Option<String>,
    pub drop_history_count: u64,
}

impl DragDropZone {
    pub fn new(source_id: impl Into<String>, target_id: impl Into<String>) -> Self {
        Self {
            source_id: source_id.into(),
            target_id: target_id.into(),
            current_state: DragState::Idle,
            payload_data: None,
            drop_history_count: 0,
        }
    }

    pub fn start_drag(&mut self, payload: impl Into<String>) {
        self.current_state = DragState::Dragging;
        self.payload_data = Some(payload.into());
    }

    pub fn complete_drop(&mut self) -> Option<String> {
        if self.current_state == DragState::Dragging {
            self.current_state = DragState::DroppedSuccess;
            self.drop_history_count += 1;
            self.payload_data.clone()
        } else {
            None
        }
    }

    pub fn cancel_drag(&mut self) {
        self.current_state = DragState::DroppedCancelled;
        self.payload_data = None;
    }
}
