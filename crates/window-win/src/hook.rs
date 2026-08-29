#[cfg(windows)]
pub mod windows_hook {
    use crossbeam_channel::Sender;
    use parking_lot::RwLock;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::thread::{self, JoinHandle};
    use tracing::{error, info};
    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, PostThreadMessageW, TranslateMessage, MSG,
        EVENT_OBJECT_CREATE, EVENT_OBJECT_DESTROY, EVENT_OBJECT_LOCATIONCHANGE,
        EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_MINIMIZEEND, EVENT_SYSTEM_MINIMIZESTART,
        OBJID_WINDOW, WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS, WM_QUIT,
    };

    #[derive(Debug, Clone)]
    pub struct RawWinEventMsg {
        pub event_type: u32,
        pub hwnd: u64,
        pub id_object: i32,
        pub id_child: i32,
        pub event_time: u32,
    }

    static WIN_EVENT_SENDER: RwLock<Option<Sender<RawWinEventMsg>>> = RwLock::new(None);

    unsafe extern "system" fn win_event_callback(
        _hook: HWINEVENTHOOK,
        event: u32,
        hwnd: HWND,
        id_object: i32,
        id_child: i32,
        _event_thread: u32,
        event_time: u32,
    ) {
        // Only track top-level window objects
        if id_object == OBJID_WINDOW.0 && id_child == 0 && !hwnd.0.is_null() {
            if let Some(ref sender) = *WIN_EVENT_SENDER.read() {
                let _ = sender.try_send(RawWinEventMsg {
                    event_type: event,
                    hwnd: hwnd.0 as u64,
                    id_object,
                    id_child,
                    event_time,
                });
            }
        }
    }

    pub struct WinEventHookThread {
        thread_id: u32,
        running: Arc<AtomicBool>,
        join_handle: Option<JoinHandle<()>>,
    }

    impl WinEventHookThread {
        pub fn start(tx: Sender<RawWinEventMsg>) -> Result<Self, String> {
            *WIN_EVENT_SENDER.write() = Some(tx);

            let (tid_tx, tid_rx) = crossbeam_channel::bounded(1);
            let running = Arc::new(AtomicBool::new(true));
            let running_clone = running.clone();

            let handle = thread::spawn(move || {
                let current_tid = unsafe { windows::Win32::System::Threading::GetCurrentThreadId() };
                let _ = tid_tx.send(current_tid);

                let hook1: HWINEVENTHOOK;
                let hook2: HWINEVENTHOOK;

                unsafe {
                    // Hook 1: Foreground & Minimize/Restore (0x0003 .. 0x0017)
                    hook1 = SetWinEventHook(
                        EVENT_SYSTEM_FOREGROUND,
                        EVENT_SYSTEM_MINIMIZEEND,
                        windows::Win32::Foundation::HINSTANCE::default(),
                        Some(win_event_callback),
                        0,
                        0,
                        WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
                    );
                    if hook1.is_invalid() {
                        error!("Failed to install SetWinEventHook 1");
                        return;
                    }

                    // Hook 2: Object Create, Destroy, Location Change (0x8000 .. 0x800B)
                    hook2 = SetWinEventHook(
                        EVENT_OBJECT_CREATE,
                        EVENT_OBJECT_LOCATIONCHANGE,
                        windows::Win32::Foundation::HINSTANCE::default(),
                        Some(win_event_callback),
                        0,
                        0,
                        WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
                    );
                    if hook2.is_invalid() {
                        error!("Failed to install SetWinEventHook 2");
                        let _ = UnhookWinEvent(hook1);
                        return;
                    }
                }

                info!("WinEvent hooks installed successfully on thread {}", current_tid);

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
                    let _ = UnhookWinEvent(hook1);
                    let _ = UnhookWinEvent(hook2);
                }

                info!("WinEvent hooks uninstalled cleanly.");
            });

            let thread_id = tid_rx
                .recv_timeout(std::time::Duration::from_secs(2))
                .map_err(|e| format!("Timed out starting WinEvent hook thread: {}", e))?;

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
                *WIN_EVENT_SENDER.write() = None;
            }
        }
    }

    impl Drop for WinEventHookThread {
        fn drop(&mut self) {
            self.stop();
        }
    }
}
