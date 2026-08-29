use core_types::metadata::BoundingRect;
use serde::{Deserialize, Serialize};

/// Detailed snapshot of an active or tracked window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WindowState {
    pub hwnd: u64,
    pub pid: u32,
    pub process_name: String,
    pub exe_path: String,
    pub title: String,
    pub bounds: BoundingRect,
    pub monitor_id: u32,
    pub dpi: u32,
    pub is_minimized: bool,
    pub is_maximized: bool,
    pub is_foreground: bool,
}

/// Information about an active physical display monitor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonitorInfo {
    pub monitor_id: u32,
    pub bounds: BoundingRect,
    pub is_primary: bool,
    pub dpi: u32,
    pub device_name: String,
}

impl MonitorInfo {
    pub fn new(
        monitor_id: u32,
        bounds: BoundingRect,
        is_primary: bool,
        dpi: u32,
        device_name: impl Into<String>,
    ) -> Self {
        Self {
            monitor_id,
            bounds,
            is_primary,
            dpi,
            device_name: device_name.into(),
        }
    }
}
