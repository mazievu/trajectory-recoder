#[cfg(windows)]
pub mod native_watcher {
    use crossbeam_channel::Sender;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::thread::{self, JoinHandle};
    use tracing::{error, info, warn};
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, ReadDirectoryChangesW, FILE_ACTION_ADDED, FILE_ACTION_MODIFIED,
        FILE_ACTION_REMOVED, FILE_ACTION_RENAMED_NEW_NAME, FILE_ACTION_RENAMED_OLD_NAME,
        FILE_FLAG_BACKUP_SEMANTICS, FILE_LIST_DIRECTORY, FILE_NOTIFY_CHANGE_CREATION,
        FILE_NOTIFY_CHANGE_DIR_NAME, FILE_NOTIFY_CHANGE_FILE_NAME, FILE_NOTIFY_CHANGE_LAST_WRITE,
        FILE_NOTIFY_CHANGE_SIZE, FILE_NOTIFY_INFORMATION, FILE_SHARE_DELETE, FILE_SHARE_READ,
        FILE_SHARE_WRITE, OPEN_EXISTING,
    };

    #[derive(Debug, Clone)]
    pub struct RawFileChangeMsg {
        pub action: String,
        pub file_path: String,
        pub old_file_path: Option<String>,
    }

    pub struct DirectoryWatcherThread {
        dir_path: PathBuf,
        running: Arc<AtomicBool>,
        join_handle: Option<JoinHandle<()>>,
    }

    impl DirectoryWatcherThread {
        pub fn start(dir: impl Into<PathBuf>, tx: Sender<RawFileChangeMsg>) -> Result<Self, String> {
            let dir_path = dir.into();
            if !dir_path.exists() {
                let _ = std::fs::create_dir_all(&dir_path);
            }

            let running = Arc::new(AtomicBool::new(true));
            let running_clone = running.clone();
            let watch_dir = dir_path.clone();

            let handle = thread::spawn(move || {
                let wide_path: Vec<u16> = watch_dir
                    .to_string_lossy()
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .collect();

                let h_dir = unsafe {
                    CreateFileW(
                        PCWSTR(wide_path.as_ptr()),
                        FILE_LIST_DIRECTORY.0,
                        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                        None,
                        OPEN_EXISTING,
                        FILE_FLAG_BACKUP_SEMANTICS,
                        HANDLE::default(),
                    )
                };

                let h_dir = match h_dir {
                    Ok(h) if h != INVALID_HANDLE_VALUE => h,
                    _ => {
                        error!("Failed to open directory handle for watching: {:?}", watch_dir);
                        return;
                    }
                };

                info!("ReadDirectoryChangesW watcher started on {:?}", watch_dir);

                let mut buffer = vec![0u8; 65536];
                let mut bytes_returned = 0u32;
                let mut pending_rename_old: Option<String> = None;

                let filter = FILE_NOTIFY_CHANGE_FILE_NAME
                    | FILE_NOTIFY_CHANGE_DIR_NAME
                    | FILE_NOTIFY_CHANGE_LAST_WRITE
                    | FILE_NOTIFY_CHANGE_SIZE
                    | FILE_NOTIFY_CHANGE_CREATION;

                while running_clone.load(Ordering::Relaxed) {
                    let success = unsafe {
                        ReadDirectoryChangesW(
                            h_dir,
                            buffer.as_mut_ptr() as *mut _,
                            buffer.len() as u32,
                            true, // Watch subtrees
                            filter,
                            Some(&mut bytes_returned),
                            None,
                            None,
                        )
                    };

                    if success.is_err() || bytes_returned == 0 {
                        thread::sleep(std::time::Duration::from_millis(50));
                        continue;
                    }

                    let mut offset = 0usize;
                    loop {
                        if offset >= bytes_returned as usize {
                            break;
                        }

                        let info_ptr = unsafe {
                            &*(buffer.as_ptr().add(offset) as *const FILE_NOTIFY_INFORMATION)
                        };

                        let name_len = (info_ptr.FileNameLength as usize) / 2;
                        let name_slice = unsafe {
                            std::slice::from_raw_parts(info_ptr.FileName.as_ptr(), name_len)
                        };
                        let rel_name = String::from_utf16_lossy(name_slice);
                        let full_path = watch_dir.join(&rel_name).to_string_lossy().to_string();

                        match info_ptr.Action {
                            FILE_ACTION_ADDED => {
                                let _ = tx.try_send(RawFileChangeMsg {
                                    action: "CREATED".into(),
                                    file_path: full_path,
                                    old_file_path: None,
                                });
                            }
                            FILE_ACTION_REMOVED => {
                                let _ = tx.try_send(RawFileChangeMsg {
                                    action: "DELETED".into(),
                                    file_path: full_path,
                                    old_file_path: None,
                                });
                            }
                            FILE_ACTION_MODIFIED => {
                                let _ = tx.try_send(RawFileChangeMsg {
                                    action: "MODIFIED".into(),
                                    file_path: full_path,
                                    old_file_path: None,
                                });
                            }
                            FILE_ACTION_RENAMED_OLD_NAME => {
                                pending_rename_old = Some(full_path);
                            }
                            FILE_ACTION_RENAMED_NEW_NAME => {
                                let old_path = pending_rename_old.take();
                                let _ = tx.try_send(RawFileChangeMsg {
                                    action: "RENAMED".into(),
                                    file_path: full_path,
                                    old_file_path: old_path,
                                });
                            }
                            _ => {}
                        }

                        if info_ptr.NextEntryOffset == 0 {
                            break;
                        }
                        offset += info_ptr.NextEntryOffset as usize;
                    }
                }

                unsafe {
                    let _ = CloseHandle(h_dir);
                }
                info!("ReadDirectoryChangesW watcher stopped for {:?}", watch_dir);
            });

            Ok(Self {
                dir_path,
                running,
                join_handle: Some(handle),
            })
        }

        pub fn stop(&mut self) {
            if self.running.swap(false, Ordering::SeqCst) {
                if let Some(h) = self.join_handle.take() {
                    let _ = h.join();
                }
            }
        }
    }

    impl Drop for DirectoryWatcherThread {
        fn drop(&mut self) {
            self.stop();
        }
    }
}
