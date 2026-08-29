use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrollControl {
    pub automation_id: String,
    pub name: String,
    pub viewport_width: f64,
    pub viewport_height: f64,
    pub content_width: f64,
    pub content_height: f64,
    pub offset_x: f64,
    pub offset_y: f64,
    pub total_scroll_events: u64,
}

impl ScrollControl {
    pub fn new(
        automation_id: impl Into<String>,
        name: impl Into<String>,
        viewport_w: f64,
        viewport_h: f64,
        content_w: f64,
        content_h: f64,
    ) -> Self {
        Self {
            automation_id: automation_id.into(),
            name: name.into(),
            viewport_width: viewport_w,
            viewport_height: viewport_h,
            content_width: content_w,
            content_height: content_h,
            offset_x: 0.0,
            offset_y: 0.0,
            total_scroll_events: 0,
        }
    }

    pub fn scroll(&mut self, delta_x: f64, delta_y: f64) {
        self.total_scroll_events += 1;
        self.offset_x = (self.offset_x + delta_x)
            .clamp(0.0, (self.content_width - self.viewport_width).max(0.0));
        self.offset_y = (self.offset_y + delta_y)
            .clamp(0.0, (self.content_height - self.viewport_height).max(0.0));
    }
}
