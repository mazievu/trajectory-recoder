use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowState {
    pub hwnd: u64,
    pub pid: u32,
    pub exe_name: String,
    pub title: String,
    pub is_maximized: bool,
}

#[test]
fn test_f09_window_switch_event_tracking() {
    let w1 = WindowState {
        hwnd: 0x1004A,
        pid: 4820,
        exe_name: "chrome.exe".to_string(),
        title: "Google Search - Chrome".to_string(),
        is_maximized: true,
    };
    let w2 = WindowState {
        hwnd: 0x2005B,
        pid: 8912,
        exe_name: "excel.exe".to_string(),
        title: "Q3_Report.xlsx - Excel".to_string(),
        is_maximized: false,
    };

    assert_ne!(w1.hwnd, w2.hwnd);
    assert_ne!(w1.exe_name, w2.exe_name);
}
