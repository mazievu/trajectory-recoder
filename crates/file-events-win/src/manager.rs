use crate::dialog::FileDialogEvent;
use crate::filter::is_noise_file;
use core_types::event::{EventSource, RawEvent, RawEventPayload, RawFileEvent};
use core_types::id::GlobalEventId;
use core_types::timestamp::DualTimestamp;
use crossbeam_channel::{Receiver, Sender, bounded};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tracing::{info, warn};

pub struct FileWatcherManager {
    machine_id: String,
    windows_session_id: u32,
    user_id: String,
    event_seq: Arc<AtomicU64>,
    global_seq: Arc<AtomicU64>,
    output_tx: Sender<RawEvent>,
    output_rx: Receiver<RawEvent>,
    running: Arc<AtomicBool>,
    worker_handle: Option<JoinHandle<()>>,
    #[cfg(windows)]
    watchers: Vec<crate::watcher::native_watcher::DirectoryWatcherThread>,
    is_mock: bool,
}

impl FileWatcherManager {
    /// Start watching user directories (e.g. Documents, Downloads, Desktop) using ReadDirectoryChangesW.
    pub fn start(
        machine_id: impl Into<String>,
        windows_session_id: u32,
        user_id: impl Into<String>,
        watch_dirs: Vec<PathBuf>,
    ) -> Result<Self, String> {
        let (out_tx, out_rx) = bounded(10_000);
        let machine_id = machine_id.into();
        let user_id = user_id.into();
        let running = Arc::new(AtomicBool::new(true));

        #[cfg(windows)]
        {
            let (raw_tx, raw_rx) = bounded(5_000);
            let mut watchers = Vec::new();
            let mut is_mock = true;

            for dir in watch_dirs {
                match crate::watcher::native_watcher::DirectoryWatcherThread::start(
                    dir.clone(),
                    raw_tx.clone(),
                ) {
                    Ok(w) => {
                        watchers.push(w);
                        is_mock = false;
                    }
                    Err(e) => {
                        warn!("Could not start directory watcher for {:?}: {}", dir, e);
                    }
                }
            }

            let mut mgr = Self {
                machine_id,
                windows_session_id,
                user_id,
                event_seq: Arc::new(AtomicU64::new(1)),
                global_seq: Arc::new(AtomicU64::new(1)),
                output_tx: out_tx.clone(),
                output_rx: out_rx,
                running: running.clone(),
                worker_handle: None,
                watchers,
                is_mock,
            };

            if !is_mock {
                let m_id = mgr.machine_id.clone();
                let u_id = mgr.user_id.clone();
                let sess_id = mgr.windows_session_id;
                let ev_seq = mgr.event_seq.clone();
                let g_seq = mgr.global_seq.clone();
                let r_clone = running.clone();

                let worker = thread::spawn(move || {
                    while r_clone.load(Ordering::Relaxed) {
                        match raw_rx.recv_timeout(Duration::from_millis(50)) {
                            Ok(msg) => {
                                if is_noise_file(&msg.file_path) {
                                    continue;
                                }

                                let payload = RawEventPayload::File(RawFileEvent {
                                    action: msg.action,
                                    file_path: msg.file_path,
                                    old_file_path: msg.old_file_path,
                                });

                                let seq = ev_seq.fetch_add(1, Ordering::Relaxed);
                                let gseq = g_seq.fetch_add(1, Ordering::Relaxed);
                                let raw_event = RawEvent::new(
                                    seq,
                                    GlobalEventId::new(gseq),
                                    DualTimestamp::now(),
                                    m_id.clone(),
                                    sess_id,
                                    u_id.clone(),
                                    EventSource::FileWatcher,
                                    seq,
                                    payload,
                                );
                                let _ = out_tx.try_send(raw_event);
                            }
                            Err(_) => {}
                        }
                    }
                });
                mgr.worker_handle = Some(worker);
            }

            Ok(mgr)
        }

        #[cfg(not(windows))]
        {
            Ok(Self::start_mock(machine_id, windows_session_id, user_id))
        }
    }

    /// Start in explicit mock mode.
    pub fn start_mock(
        machine_id: impl Into<String>,
        windows_session_id: u32,
        user_id: impl Into<String>,
    ) -> Self {
        let (out_tx, out_rx) = bounded(10_000);
        Self {
            machine_id: machine_id.into(),
            windows_session_id,
            user_id: user_id.into(),
            event_seq: Arc::new(AtomicU64::new(1)),
            global_seq: Arc::new(AtomicU64::new(1)),
            output_tx: out_tx,
            output_rx: out_rx,
            running: Arc::new(AtomicBool::new(true)),
            worker_handle: None,
            #[cfg(windows)]
            watchers: Vec::new(),
            is_mock: true,
        }
    }

    pub fn is_mock(&self) -> bool {
        self.is_mock
    }

    pub fn receiver(&self) -> Receiver<RawEvent> {
        self.output_rx.clone()
    }

    /// Simulate file activity (e.g. CREATED, MODIFIED, DELETED, RENAMED).
    pub fn simulate_file_event(
        &self,
        action: impl Into<String>,
        file_path: impl Into<String>,
        old_file_path: Option<String>,
    ) {
        let path = file_path.into();
        if is_noise_file(&path) {
            return;
        }

        let payload = RawEventPayload::File(RawFileEvent {
            action: action.into(),
            file_path: path,
            old_file_path,
        });

        let seq = self.event_seq.fetch_add(1, Ordering::Relaxed);
        let gseq = self.global_seq.fetch_add(1, Ordering::Relaxed);
        let raw_event = RawEvent::new(
            seq,
            GlobalEventId::new(gseq),
            DualTimestamp::now(),
            self.machine_id.clone(),
            self.windows_session_id,
            self.user_id.clone(),
            EventSource::FileWatcher,
            seq,
            payload,
        );
        let _ = self.output_tx.try_send(raw_event);
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        #[cfg(windows)]
        for watcher in self.watchers.iter_mut() {
            watcher.stop();
        }
        if let Some(h) = self.worker_handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for FileWatcherManager {
    fn drop(&mut self) {
        self.stop();
    }
}
