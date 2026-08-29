#[cfg(windows)]
pub mod native_listener {
    use crate::formats::format_id_to_name;
    use crate::hasher::compute_sha256;
    use crossbeam_channel::Sender;
    use parking_lot::RwLock;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::thread::{self, JoinHandle};
    use tracing::{error, info, warn};
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::DataExchange::{
        AddClipboardFormatListener, CloseClipboard, EnumClipboardFormats, GetClipboardData,
        GetClipboardFormatNameW, GetClipboardOwner, OpenClipboard, RemoveClipboardFormatListener,
    };
    use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
        PostMessageW, RegisterClassExW, TranslateMessage, CS_HREDRAW, CS_VREDRAW, HMENU, MSG,
        WINDOW_EX_STYLE, WM_CLIPBOARDUPDATE, WM_DESTROY, WM_QUIT, WNDCLASSEXW, WS_OVERLAPPEDWINDOW,
    };

    #[derive(Debug, Clone)]
    pub struct ClipboardMetadataMsg {
        pub format: String,
        pub byte_length: usize,
        pub hash_sha256: String,
        pub source_hwnd: Option<u64>,
    }

    static CLIPBOARD_SENDER: RwLock<Option<Sender<ClipboardMetadataMsg>>> = RwLock::new(None);

    unsafe extern "system" fn clipboard_wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if msg == WM_CLIPBOARDUPDATE {
            let metadata = inspect_clipboard(hwnd);
            if let Some(meta) = metadata {
                if let Some(ref sender) = *CLIPBOARD_SENDER.read() {
                    let _ = sender.try_send(meta);
                }
            }
            return LRESULT(0);
        } else if msg == WM_DESTROY {
            let _ = RemoveClipboardFormatListener(hwnd);
            return LRESULT(0);
        }

        DefWindowProcW(hwnd, msg, wparam, lparam)
    }

    fn inspect_clipboard(hwnd: HWND) -> Option<ClipboardMetadataMsg> {
        let mut opened = false;
        for _ in 0..5 {
            if unsafe { OpenClipboard(hwnd).is_ok() } {
                opened = true;
                break;
            }
            thread::sleep(std::time::Duration::from_millis(10));
        }

        if !opened {
            return None;
        }

        let mut primary_format = String::from("UNKNOWN");
        let mut byte_length = 0usize;
        let mut hash_sha256 = String::new();
        let mut source_hwnd = None;

        unsafe {
            if let Ok(owner) = GetClipboardOwner() {
                if !owner.0.is_null() {
                    source_hwnd = Some(owner.0 as u64);
                }
            }

            // Enumerate available formats
            let mut fmt = EnumClipboardFormats(0);
            let mut formats = Vec::new();
            while fmt != 0 {
                formats.push(fmt);
                fmt = EnumClipboardFormats(fmt);
            }

            if let Some(&first_fmt) = formats.first() {
                // Determine format name
                let mut name_buf = [0u16; 256];
                let name_len = GetClipboardFormatNameW(first_fmt, &mut name_buf);
                primary_format = if name_len > 0 {
                    String::from_utf16_lossy(&name_buf[..name_len as usize])
                } else {
                    format_id_to_name(first_fmt)
                };

                // Read data handle safely to hash and measure length (fail-closed: do not save raw text)
                if let Ok(handle) = GetClipboardData(first_fmt) {
                    if !handle.0.is_null() {
                        let size = GlobalSize(windows::Win32::Foundation::HGLOBAL(handle.0 as *mut _));
                        byte_length = size;

                        let ptr = GlobalLock(windows::Win32::Foundation::HGLOBAL(handle.0 as *mut _));
                        if !ptr.is_null() && size > 0 {
                            let slice = std::slice::from_raw_parts(ptr as *const u8, size);
                            hash_sha256 = compute_sha256(slice);
                            let _ = GlobalUnlock(windows::Win32::Foundation::HGLOBAL(handle.0 as *mut _));
                        }
                    }
                }
            }

            let _ = CloseClipboard();
        }

        if hash_sha256.is_empty() {
            hash_sha256 = compute_sha256(b"");
        }

        Some(ClipboardMetadataMsg {
            format: primary_format,
            byte_length,
            hash_sha256,
            source_hwnd,
        })
    }

    pub struct ClipboardListenerThread {
        hwnd: u64,
        running: Arc<AtomicBool>,
        join_handle: Option<JoinHandle<()>>,
    }

    impl ClipboardListenerThread {
        pub fn start(tx: Sender<ClipboardMetadataMsg>) -> Result<Self, String> {
            *CLIPBOARD_SENDER.write() = Some(tx);

            let (hwnd_tx, hwnd_rx) = crossbeam_channel::bounded(1);
            let running = Arc::new(AtomicBool::new(true));
            let running_clone = running.clone();

            let handle = thread::spawn(move || {
                let class_name: Vec<u16> = "TrajectoryClipboardWatcherClass\0".encode_utf16().collect();
                let window_title: Vec<u16> = "TrajectoryClipboardWatcher\0".encode_utf16().collect();

                let hinstance = HINSTANCE::default();
                let wc = WNDCLASSEXW {
                    cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                    style: CS_HREDRAW | CS_VREDRAW,
                    lpfnWndProc: Some(clipboard_wnd_proc),
                    cbClsExtra: 0,
                    cbWndExtra: 0,
                    hInstance: hinstance,
                    hIcon: Default::default(),
                    hCursor: Default::default(),
                    hbrBackground: Default::default(),
                    lpszMenuName: PCWSTR::null(),
                    lpszClassName: PCWSTR(class_name.as_ptr()),
                    hIconSm: Default::default(),
                };

                unsafe {
                    let _ = RegisterClassExW(&wc);
                    let hwnd = CreateWindowExW(
                        WINDOW_EX_STYLE(0),
                        PCWSTR(class_name.as_ptr()),
                        PCWSTR(window_title.as_ptr()),
                        WS_OVERLAPPEDWINDOW,
                        0,
                        0,
                        0,
                        0,
                        HWND::default(),
                        HMENU::default(),
                        hinstance,
                        None,
                    );

                    match hwnd {
                        Ok(h) if !h.0.is_null() => {
                            if AddClipboardFormatListener(h).is_ok() {
                                info!("AddClipboardFormatListener attached successfully to HWND {:?}", h.0);
                                let _ = hwnd_tx.send(Ok(h.0 as u64));
                            } else {
                                let _ = hwnd_tx.send(Err("AddClipboardFormatListener failed".to_string()));
                                return;
                            }
                        }
                        _ => {
                            let _ = hwnd_tx.send(Err("CreateWindowExW failed".to_string()));
                            return;
                        }
                    }
                }

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

                info!("Clipboard listener thread exited cleanly.");
            });

            let hwnd = match hwnd_rx.recv_timeout(std::time::Duration::from_secs(2)) {
                Ok(Ok(h)) => h,
                Ok(Err(e)) => return Err(e),
                Err(e) => return Err(format!("Timed out starting clipboard window: {}", e)),
            };

            Ok(Self {
                hwnd,
                running,
                join_handle: Some(handle),
            })
        }

        pub fn stop(&mut self) {
            if self.running.swap(false, Ordering::SeqCst) {
                unsafe {
                    let _ = PostMessageW(HWND(self.hwnd as *mut _), WM_QUIT, WPARAM(0), LPARAM(0));
                    let _ = DestroyWindow(HWND(self.hwnd as *mut _));
                }
                if let Some(handle) = self.join_handle.take() {
                    let _ = handle.join();
                }
                *CLIPBOARD_SENDER.write() = None;
            }
        }
    }

    impl Drop for ClipboardListenerThread {
        fn drop(&mut self) {
            self.stop();
        }
    }
}
