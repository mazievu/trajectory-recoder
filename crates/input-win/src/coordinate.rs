use core_types::metadata::{BoundingRect, Point2D};
use parking_lot::RwLock;
use std::sync::Arc;

/// Geometry of a single display monitor.
#[derive(Debug, Clone, PartialEq)]
pub struct MonitorBounds {
    pub monitor_id: u32,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub is_primary: bool,
}

impl MonitorBounds {
    pub fn new(monitor_id: u32, x: i32, y: i32, width: u32, height: u32, is_primary: bool) -> Self {
        Self {
            monitor_id,
            x,
            y,
            width,
            height,
            is_primary,
        }
    }

    /// Check if physical point falls within this monitor.
    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x
            && px < self.x + self.width as i32
            && py >= self.y
            && py < self.y + self.height as i32
    }

    /// Normalize a physical point to [0.0, 1.0] relative to this monitor.
    pub fn normalize(&self, px: i32, py: i32) -> (f32, f32) {
        let w = if self.width > 0 {
            self.width as f32
        } else {
            1.0
        };
        let h = if self.height > 0 {
            self.height as f32
        } else {
            1.0
        };

        let nx = ((px - self.x) as f32 / w).clamp(0.0, 1.0);
        let ny = ((py - self.y) as f32 / h).clamp(0.0, 1.0);
        (nx, ny)
    }

    pub fn to_bounding_rect(&self) -> BoundingRect {
        BoundingRect::new(
            self.x,
            self.y,
            self.x + self.width as i32,
            self.y + self.height as i32,
        )
    }
}

/// Multi-monitor coordinate mapper supporting normalized coordinate projection.
#[derive(Debug, Clone)]
pub struct CoordinateMapper {
    monitors: Arc<RwLock<Vec<MonitorBounds>>>,
}

impl Default for CoordinateMapper {
    fn default() -> Self {
        Self::new()
    }
}

impl CoordinateMapper {
    pub fn new() -> Self {
        // Default to a standard 1920x1080 primary monitor
        let default_monitor = MonitorBounds::new(0, 0, 0, 1920, 1080, true);
        Self {
            monitors: Arc::new(RwLock::new(vec![default_monitor])),
        }
    }

    pub fn with_monitors(monitors: Vec<MonitorBounds>) -> Self {
        let list = if monitors.is_empty() {
            vec![MonitorBounds::new(0, 0, 0, 1920, 1080, true)]
        } else {
            monitors
        };
        Self {
            monitors: Arc::new(RwLock::new(list)),
        }
    }

    pub fn update_monitors(&self, monitors: Vec<MonitorBounds>) {
        if !monitors.is_empty() {
            *self.monitors.write() = monitors;
        }
    }

    /// Map physical coordinates (px, py) to (monitor_id, normalized_x, normalized_y, Point2D).
    pub fn map_point(&self, px: i32, py: i32) -> (u32, f32, f32, Point2D) {
        let monitors = self.monitors.read();

        // 1. Try to find the exact monitor containing the point
        for mon in monitors.iter() {
            if mon.contains(px, py) {
                let (nx, ny) = mon.normalize(px, py);
                return (mon.monitor_id, nx, ny, Point2D::new(px, py, nx, ny));
            }
        }

        // 2. Fallback to primary monitor or first monitor
        if let Some(primary) = monitors
            .iter()
            .find(|m| m.is_primary)
            .or_else(|| monitors.first())
        {
            let (nx, ny) = primary.normalize(px, py);
            (primary.monitor_id, nx, ny, Point2D::new(px, py, nx, ny))
        } else {
            (0, 0.0, 0.0, Point2D::new(px, py, 0.0, 0.0))
        }
    }

    /// Get current monitor list snapshot.
    pub fn get_monitors(&self) -> Vec<MonitorBounds> {
        self.monitors.read().clone()
    }
}
