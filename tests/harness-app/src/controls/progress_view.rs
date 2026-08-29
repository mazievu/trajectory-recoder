use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressControl {
    pub automation_id: String,
    pub name: String,
    pub is_indeterminate: bool,
    pub progress_percent: f64, // 0.0 .. 100.0
    pub is_active: bool,
    pub simulated_latency_ms: u64,
}

impl ProgressControl {
    pub fn new(automation_id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            automation_id: automation_id.into(),
            name: name.into(),
            is_indeterminate: true,
            progress_percent: 0.0,
            is_active: false,
            simulated_latency_ms: 1800,
        }
    }

    pub fn start(&mut self, latency_ms: u64) {
        self.is_active = true;
        self.simulated_latency_ms = latency_ms;
        self.progress_percent = 0.0;
    }

    pub fn set_progress(&mut self, percent: f64) {
        self.is_indeterminate = false;
        self.progress_percent = percent.clamp(0.0, 100.0);
    }

    pub fn finish(&mut self) {
        self.is_active = false;
        self.progress_percent = 100.0;
    }
}
