#[cfg(windows)]
pub mod windows_hook {
    use core_types::metadata::MouseButton;
    use crossbeam_channel::Sender;
    use parking_lot::RwLock;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::thread::{self, JoinHandle};
    use tracing::{error, info, warn};
    use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetMessageW, PostThreadMessageW, SetWindowsHookExW,
        TranslateMessage, UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT, MSLLHOOKSTRUCT, MSG,
        WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_LBUTTONUP,
        WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEHWHEEL, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_QUIT,
        WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_XBUTTONDOWN, WM_XBUTTONUP,
    };

    #[derive(Debug, Clone)]
    pub struct RawMouseHookMsg {
        pub event_type: &'static str,
        pub button: MouseButton,
        pub px: i32,
        pub py: i32,
        pub delta_x: f64,
        pub delta_y: f64,
        pub time: u32,
    }

    #[derive(Debug, Clone)]
    pub struct RawKeyboardHookMsg {
        pub event_type: &'static str,
        pub vk_code: u32,
        pub scan_code: u32,
        pub is_down: bool,
        pub is_injected: bool,
        pub time: u32,
    }

    // Static global senders for hook procs
    static MOUSE_SENDER: RwLock<Option<Sender<RawMouseHookMsg>>> = RwLock::new(None);
    static KEYBOARD_SENDER: RwLock<Option<Sender<RawKeyboardHookMsg>>> = RwLock::new(None);

    unsafe extern "system" fn low_level_mouse_proc(
        ncode: i32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if ncode >= 0 {
            let hook_struct = unsafe { *(lparam.0 as *const MSLLHOOKSTRUCT) };
            let msg_type = wparam.0 as u32;

            let (event_type, button, delta_x, delta_y) = match msg_type {
                WM_LBUTTONDOWN => ("MOUSE_DOWN", MouseButton::Left, 0.0, 0.0),
                WM_LBUTTONUP => ("MOUSE_UP", MouseButton::Left, 0.0, 0.0),
                WM_RBUTTONDOWN => ("MOUSE_DOWN", MouseButton::Right, 0.0, 0.0),
                WM_RBUTTONUP => ("MOUSE_UP", MouseButton::Right, 0.0, 0.0),
                WM_MBUTTONDOWN => ("MOUSE_DOWN", MouseButton::Middle, 0.0, 0.0),
                WM_MBUTTONUP => ("MOUSE_UP", MouseButton::Middle, 0.0, 0.0),
                WM_MOUSEMOVE => ("MOUSE_MOVE", MouseButton::None, 0.0, 0.0),
                WM_MOUSEWHEEL => {
                    let delta = ((hook_struct.mouseData >> 16) as i16) as f64;
                    ("MOUSE_WHEEL", MouseButton::None, 0.0, delta)
                }
                WM_MOUSEHWHEEL => {
                    let delta = ((hook_struct.mouseData >> 16) as i16) as f64;
                    ("MOUSE_WHEEL", MouseButton::None, delta, 0.0)
                }
                WM_XBUTTONDOWN => {
                    let btn = if (hook_struct.mouseData >> 16) == 1 {
                        MouseButton::X1
                    } else {
                        MouseButton::X2
                    };
                    ("MOUSE_DOWN", btn, 0.0, 0.0)
                }
                WM_XBUTTONUP => {
                    let btn = if (hook_struct.mouseData >> 16) == 1 {
                        MouseButton::X1
                    } else {
                        MouseButton::X2
                    };
                    ("MOUSE_UP", btn, 0.0, 0.0)
                }
                _ => ("UNKNOWN", MouseButton::None, 0.0, 0.0),
            };

            if event_type != "UNKNOWN" {
                if let Some(ref sender) = *MOUSE_SENDER.read() {
                    let _ = sender.try_send(RawMouseHookMsg {
                        event_type,
                        button,
                        px: hook_struct.pt.x,
                        py: hook_struct.pt.y,
                        delta_x,
                        delta_y,
                        time: hook_struct.time,
                    });
                }
            }
        }

        unsafe { CallNextHookEx(None, ncode, wparam, lparam) }
    }

    unsafe extern "system" fn low_level_keyboard_proc(
        ncode: i32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if ncode >= 0 {
            let hook_struct = unsafe { *(lparam.0 as *const KBDLLHOOKSTRUCT) };
            let msg_type = wparam.0 as u32;

            let (event_type, is_down) = match msg_type {
                WM_KEYDOWN | WM_SYSKEYDOWN => ("KEY_DOWN", true),
                WM_KEYUP | WM_SYSKEYUP => ("KEY_UP", false),
                _ => ("UNKNOWN", false),
            };

            if event_type != "UNKNOWN" {
                let is_injected = (hook_struct.flags.0 & 0x01) != 0;
                if let Some(ref sender) = *KEYBOARD_SENDER.read() {
                    let _ = sender.try_send(RawKeyboardHookMsg {
                        event_type,
                        vk_code: hook_struct.vkCode,
                        scan_code: hook_struct.scanCode,
                        is_down,
                        is_injected,
                        time: hook_struct.time,
                    });
                }
            }
        }

        unsafe { CallNextHookEx(None, ncode, wparam, lparam) }
    }

    /// Dedicated Win32 message pump thread running WH_MOUSE_LL and WH_KEYBOARD_LL hooks.
    pub struct Win32HookThread {
        thread_id: u32,
        running: Arc<AtomicBool>,
        join_handle: Option<JoinHandle<()>>,
    }

    impl Win32HookThread {
        pub fn start(
            mouse_tx: Sender<RawMouseHookMsg>,
            keyboard_tx: Sender<RawKeyboardHookMsg>,
        ) -> Result<Self, String> {
            *MOUSE_SENDER.write() = Some(mouse_tx);
            *KEYBOARD_SENDER.write() = Some(keyboard_tx);

            let (tid_tx, tid_rx) = crossbeam_channel::bounded(1);
            let running = Arc::new(AtomicBool::new(true));
            let running_clone = running.clone();

            let handle = thread::spawn(move || {
                let current_tid = unsafe { windows::Win32::System::Threading::GetCurrentThreadId() };
                let _ = tid_tx.send(current_tid);

                let mouse_hook: HHOOK;
                let keyboard_hook: HHOOK;

                unsafe {
                    mouse_hook = match SetWindowsHookExW(
                        WH_MOUSE_LL,
                        Some(low_level_mouse_proc),
                        HINSTANCE::default(),
                        0,
                    ) {
                        Ok(h) => h,
                        Err(e) => {
                            error!("Failed to install WH_MOUSE_LL hook: {:?}", e);
                            return;
                        }
                    };

                    keyboard_hook = match SetWindowsHookExW(
                        WH_KEYBOARD_LL,
                        Some(low_level_keyboard_proc),
                        HINSTANCE::default(),
                        0,
                    ) {
                        Ok(h) => h,
                        Err(e) => {
                            error!("Failed to install WH_KEYBOARD_LL hook: {:?}", e);
                            let _ = UnhookWindowsHookEx(mouse_hook);
                            return;
                        }
                    };
                }

                info!("Win32 low-level input hooks installed successfully on thread {}", current_tid);

                let mut msg = MSG::default();
                while running_clone.load(Ordering::Relaxed) {
                    unsafe {
                        let res = GetMessageW(&mut msg, HWND::default(), 0, 0);
                        if res.0 <= 0 || msg.message == WM_QUIT {
                            break;
                        }
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }

                unsafe {
                    let _ = UnhookWindowsHookEx(mouse_hook);
                    let _ = UnhookWindowsHookEx(keyboard_hook);
                }

                info!("Win32 low-level input hooks uninstalled cleanly.");
            });

            let thread_id = tid_rx
                .recv_timeout(std::time::Duration::from_secs(2))
                .map_err(|e| format!("Timed out starting hook thread: {}", e))?;

            Ok(Self {
                thread_id,
                running,
                join_handle: Some(handle),
            })
        }

        pub fn stop(&mut self) {
            if self.running.swap(false, Ordering::SeqCst) {
                unsafe {
                    let _ = PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
                }
                if let Some(handle) = self.join_handle.take() {
                    let _ = handle.join();
                }
                *MOUSE_SENDER.write() = None;
                *KEYBOARD_SENDER.write() = None;
            }
        }
    }

    impl Drop for Win32HookThread {
        fn drop(&mut self) {
            self.stop();
        }
    }
}
