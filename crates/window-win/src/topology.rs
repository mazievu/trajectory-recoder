use crate::model::MonitorInfo;
use core_types::metadata::BoundingRect;
use parking_lot::RwLock;
use std::sync::Arc;

/// Monitor topology tracker maintaining display geometries and DPI scales.
#[derive(Debug, Clone)]
pub struct MonitorTopology {
    monitors: Arc<RwLock<Vec<MonitorInfo>>>,
}

impl Default for MonitorTopology {
    fn default() -> Self {
        Self::new()
    }
}

impl MonitorTopology {
    pub fn new() -> Self {
        let default_mon = MonitorInfo::new(
            0,
            BoundingRect::new(0, 0, 1920, 1080),
            true,
            96,
            "\\\\.\\DISPLAY1",
        );
        Self {
            monitors: Arc::new(RwLock::new(vec![default_mon])),
        }
    }

    pub fn with_monitors(monitors: Vec<MonitorInfo>) -> Self {
        let list = if monitors.is_empty() {
            vec![MonitorInfo::new(
                0,
                BoundingRect::new(0, 0, 1920, 1080),
                true,
                96,
                "\\\\.\\DISPLAY1",
            )]
        } else {
            monitors
        };
        Self {
            monitors: Arc::new(RwLock::new(list)),
        }
    }

    /// Refresh display monitor list via Win32 EnumDisplayMonitors on Windows.
    #[cfg(windows)]
    pub fn refresh_from_system(&self) {
        use windows::Win32::Foundation::{BOOL, LPARAM, RECT};
        use windows::Win32::Graphics::Gdi::{
            EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW,
        };

        let mut found_monitors = Vec::new();

        unsafe extern "system" fn enum_mon_proc(
            hmonitor: HMONITOR,
            _hdc: HDC,
            _lprect: *mut RECT,
            lparam: LPARAM,
        ) -> BOOL {
            let list = unsafe { &mut *(lparam.0 as *mut Vec<MonitorInfo>) };
            let mut info = MONITORINFOEXW::default();
            info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;

            if unsafe { GetMonitorInfoW(hmonitor, &mut info.monitorInfo as *mut _ as *mut _).as_bool() } {
                let rect = info.monitorInfo.rcMonitor;
                let is_primary = (info.monitorInfo.dwFlags & 1) != 0;

                let id = list.len() as u32;
                let dev_name = String::from_utf16_lossy(
                    &info
                        .szDevice
                        .iter()
                        .take_while(|&&c| c != 0)
                        .cloned()
                        .collect::<Vec<u16>>(),
                );

                list.push(MonitorInfo::new(
                    id,
                    BoundingRect::new(rect.left, rect.top, rect.right, rect.bottom),
                    is_primary,
                    96,
                    dev_name,
                ));
            }
            BOOL(1)
        }

        let lparam = LPARAM(&mut found_monitors as *mut _ as isize);
        unsafe {
            let _ = EnumDisplayMonitors(HDC::default(), None, Some(enum_mon_proc), lparam);
        }

        if !found_monitors.is_empty() {
            *self.monitors.write() = found_monitors;
        }
    }

    #[cfg(not(windows))]
    pub fn refresh_from_system(&self) {}

    pub fn set_monitors(&self, monitors: Vec<MonitorInfo>) {
        if !monitors.is_empty() {
            *self.monitors.write() = monitors;
        }
    }

    pub fn monitors(&self) -> Vec<MonitorInfo> {
        self.monitors.read().clone()
    }

    /// Identify which monitor contains a given window bounding rect center.
    pub fn find_monitor_for_rect(&self, rect: &BoundingRect) -> u32 {
        let center_x = rect.left + (rect.width as i32 / 2);
        let center_y = rect.top + (rect.height as i32 / 2);

        let mons = self.monitors.read();
        for mon in mons.iter() {
            if center_x >= mon.bounds.left
                && center_x < mon.bounds.right
                && center_y >= mon.bounds.top
                && center_y < mon.bounds.bottom
            {
                return mon.monitor_id;
            }
        }

        mons.iter()
            .find(|m| m.is_primary)
            .map(|m| m.monitor_id)
            .unwrap_or(0)
    }
}
