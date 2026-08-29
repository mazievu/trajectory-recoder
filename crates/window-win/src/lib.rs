//! Active window and monitor topology tracking via WinEvent hooks.

pub mod hook;
pub mod model;
pub mod topology;
pub mod tracker;
pub mod win_api;

pub use model::{MonitorInfo, WindowState};
pub use topology::MonitorTopology;
pub use tracker::WindowTracker;

#[cfg(test)]
mod tests {
    use super::*;
    use core_types::event::RawEventPayload;
    use core_types::metadata::BoundingRect;
    use std::time::Duration;

    #[test]
    fn test_monitor_topology_find_monitor() {
        let m1 = MonitorInfo::new(
            0,
            BoundingRect::new(0, 0, 1920, 1080),
            true,
            96,
            "DISPLAY1",
        );
        let m2 = MonitorInfo::new(
            1,
            BoundingRect::new(1920, 0, 3840, 1080),
            false,
            96,
            "DISPLAY2",
        );
        let topo = MonitorTopology::with_monitors(vec![m1, m2]);

        // Window on first monitor
        let w1_bounds = BoundingRect::new(100, 100, 900, 700);
        assert_eq!(topo.find_monitor_for_rect(&w1_bounds), 0);

        // Window on second monitor
        let w2_bounds = BoundingRect::new(2000, 100, 2800, 700);
        assert_eq!(topo.find_monitor_for_rect(&w2_bounds), 1);
    }

    #[test]
    fn test_window_tracker_simulation_pipeline() {
        let tracker = WindowTracker::start_mock("test_pc", 1, "test_user");
        let rx = tracker.receiver();

        let state1 = WindowState {
            hwnd: 0x1234,
            pid: 4321,
            process_name: "chrome.exe".into(),
            exe_path: "C:\\Program Files\\Google\\Chrome\\chrome.exe".into(),
            title: "New Tab - Google Chrome".into(),
            bounds: BoundingRect::new(0, 0, 1920, 1080),
            monitor_id: 0,
            dpi: 96,
            is_minimized: false,
            is_maximized: true,
            is_foreground: true,
        };

        tracker.simulate_foreground_window(state1.clone());

        assert_eq!(tracker.current_foreground().unwrap().hwnd, 0x1234);

        let event = rx.recv_timeout(Duration::from_millis(100)).unwrap();
        if let RawEventPayload::Window(w) = event.payload {
            assert_eq!(w.event_type, "FOREGROUND");
            assert_eq!(w.hwnd, 0x1234);
            assert_eq!(w.pid, 4321);
            assert_eq!(w.process_name, "chrome.exe");
            assert_eq!(w.window_title, "New Tab - Google Chrome");
            assert_eq!(w.bounds.width, 1920);
        } else {
            panic!("Expected Window payload");
        }

        // Simulate resize / move
        let state2 = WindowState {
            hwnd: 0x1234,
            pid: 4321,
            process_name: "chrome.exe".into(),
            exe_path: "C:\\Program Files\\Google\\Chrome\\chrome.exe".into(),
            title: "New Tab - Google Chrome".into(),
            bounds: BoundingRect::new(100, 100, 1300, 900),
            monitor_id: 0,
            dpi: 96,
            is_minimized: false,
            is_maximized: false,
            is_foreground: true,
        };
        tracker.simulate_window_event("MOVE", state2);

        let event2 = rx.recv_timeout(Duration::from_millis(100)).unwrap();
        if let RawEventPayload::Window(w) = event2.payload {
            assert_eq!(w.event_type, "MOVE");
            assert_eq!(w.bounds.left, 100);
            assert_eq!(w.bounds.width, 1200);
        } else {
            panic!("Expected Window payload");
        }
    }
}
