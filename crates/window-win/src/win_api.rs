#[cfg(windows)]
pub mod native {
    use crate::model::WindowState;
    use core_types::metadata::BoundingRect;
    use windows::core::PWSTR;
    use windows::Win32::Foundation::{HWND, RECT};
    use windows::Win32::System::ProcessStatus::GetModuleFileNameExW;
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION,
        PROCESS_VM_READ,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowRect, GetWindowTextLengthW, GetWindowTextW,
        GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible, IsZoomed,
    };

    pub fn get_foreground_hwnd() -> u64 {
        unsafe {
            let hwnd = GetForegroundWindow();
            hwnd.0 as u64
        }
    }

    pub fn is_valid_window(hwnd: u64) -> bool {
        unsafe { IsWindow(HWND(hwnd as *mut _)).as_bool() }
    }

    pub fn is_visible_window(hwnd: u64) -> bool {
        unsafe { IsWindowVisible(HWND(hwnd as *mut _)).as_bool() }
    }

    pub fn get_window_title(hwnd: u64) -> String {
        unsafe {
            let h = HWND(hwnd as *mut _);
            let len = GetWindowTextLengthW(h);
            if len == 0 {
                return String::new();
            }
            let mut buf = vec![0u16; (len + 1) as usize];
            let read_len = GetWindowTextW(h, &mut buf);
            if read_len > 0 {
                String::from_utf16_lossy(&buf[..read_len as usize])
            } else {
                String::new()
            }
        }
    }

    pub fn get_window_pid(hwnd: u64) -> u32 {
        unsafe {
            let mut pid = 0u32;
            GetWindowThreadProcessId(HWND(hwnd as *mut _), Some(&mut pid));
            pid
        }
    }

    pub fn get_process_info(pid: u32) -> (String, String) {
        if pid == 0 {
            return ("System".into(), String::new());
        }

        unsafe {
            let handle_res = OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ,
                false,
                pid,
            );

            if let Ok(handle) = handle_res {
                let mut path_buf = vec![0u16; 1024];
                let mut size = path_buf.len() as u32;

                let success = QueryFullProcessImageNameW(
                    handle,
                    PROCESS_NAME_FORMAT(0),
                    PWSTR(path_buf.as_mut_ptr()),
                    &mut size,
                )
                .is_ok();

                let exe_path = if success && size > 0 {
                    String::from_utf16_lossy(&path_buf[..size as usize])
                } else {
                    let mod_len = GetModuleFileNameExW(handle, None, &mut path_buf);
                    if mod_len > 0 {
                        String::from_utf16_lossy(&path_buf[..mod_len as usize])
                    } else {
                        String::new()
                    }
                };

                let process_name = if !exe_path.is_empty() {
                    std::path::Path::new(&exe_path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string()
                } else {
                    format!("PID_{}", pid)
                };

                let _ = windows::Win32::Foundation::CloseHandle(handle);
                (process_name, exe_path)
            } else {
                (format!("PID_{}", pid), String::new())
            }
        }
    }

    pub fn get_window_rect(hwnd: u64) -> BoundingRect {
        unsafe {
            let mut rect = RECT::default();
            if GetWindowRect(HWND(hwnd as *mut _), &mut rect).is_ok() {
                BoundingRect::new(rect.left, rect.top, rect.right, rect.bottom)
            } else {
                BoundingRect::new(0, 0, 0, 0)
            }
        }
    }

    pub fn get_window_dpi(hwnd: u64) -> u32 {
        unsafe {
            let dpi = windows::Win32::UI::HiDpi::GetDpiForWindow(HWND(hwnd as *mut _));
            if dpi > 0 {
                dpi
            } else {
                96
            }
        }
    }

    pub fn inspect_window(hwnd: u64, monitor_id: u32, is_foreground: bool) -> WindowState {
        let h = HWND(hwnd as *mut _);
        let pid = get_window_pid(hwnd);
        let (process_name, exe_path) = get_process_info(pid);
        let title = get_window_title(hwnd);
        let bounds = get_window_rect(hwnd);
        let dpi = get_window_dpi(hwnd);
        let is_minimized = unsafe { IsIconic(h).as_bool() };
        let is_maximized = unsafe { IsZoomed(h).as_bool() };

        WindowState {
            hwnd,
            pid,
            process_name,
            exe_path,
            title,
            bounds,
            monitor_id,
            dpi,
            is_minimized,
            is_maximized,
            is_foreground,
        }
    }
}
