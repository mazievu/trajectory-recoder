use core_types::metadata::MouseButton;

/// Double-click detector tracking temporal and spatial intervals between clicks.
#[derive(Debug, Clone)]
pub struct DoubleClickDetector {
    double_click_time_ms: u64,
    double_click_dist_px: i32,
    last_click_time_ms: u64,
    last_click_x: i32,
    last_click_y: i32,
    last_button: MouseButton,
}

impl Default for DoubleClickDetector {
    fn default() -> Self {
        Self::new(500, 4)
    }
}

impl DoubleClickDetector {
    pub fn new(double_click_time_ms: u64, double_click_dist_px: i32) -> Self {
        Self {
            double_click_time_ms,
            double_click_dist_px,
            last_click_time_ms: 0,
            last_click_x: -10000,
            last_click_y: -10000,
            last_button: MouseButton::None,
        }
    }

    /// Check if a mouse-down click is a double click.
    /// Updates internal state for subsequent detections.
    pub fn check_and_update(
        &mut self,
        button: MouseButton,
        px: i32,
        py: i32,
        current_time_ms: u64,
    ) -> bool {
        if button == MouseButton::None {
            return false;
        }

        let time_delta = current_time_ms.saturating_sub(self.last_click_time_ms);
        let dist_x = (px - self.last_click_x).abs();
        let dist_y = (py - self.last_click_y).abs();

        let is_double = button == self.last_button
            && time_delta <= self.double_click_time_ms
            && dist_x <= self.double_click_dist_px
            && dist_y <= self.double_click_dist_px;

        if is_double {
            // Reset to prevent triple-click from triggering another double click immediately
            self.last_click_time_ms = 0;
            self.last_button = MouseButton::None;
        } else {
            self.last_click_time_ms = current_time_ms;
            self.last_click_x = px;
            self.last_click_y = py;
            self.last_button = button;
        }

        is_double
    }
}
