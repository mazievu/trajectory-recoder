use crate::hasher::compute_sha256;
use core_types::event::{EventSource, RawClipboardEvent, RawEvent, RawEventPayload};
use core_types::id::GlobalEventId;
use core_types::timestamp::DualTimestamp;
use crossbeam_channel::{Receiver, Sender, bounded};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tracing::{info, warn};

pub struct ClipboardManager {
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
    listener: Option<crate::listener::native_listener::ClipboardListenerThread>,
    is_mock: bool,
}

impl ClipboardManager {
    /// Start listening to Windows clipboard format changes, or mock if running non-interactively.
    pub fn start(
        machine_id: impl Into<String>,
        windows_session_id: u32,
        user_id: impl Into<String>,
    ) -> Result<Self, String> {
        let (out_tx, out_rx) = bounded(5_000);
        let machine_id = machine_id.into();
        let user_id = user_id.into();
        let running = Arc::new(AtomicBool::new(true));

        #[cfg(windows)]
        {
            let (meta_tx, meta_rx) = bounded(1_000);
            let listener_res =
                crate::listener::native_listener::ClipboardListenerThread::start(meta_tx);
            let (listener, is_mock) = match listener_res {
                Ok(l) => (Some(l), false),
                Err(err) => {
                    warn!(
                        "Could not start clipboard format listener (fallback to simulation/mock): {}",
                        err
                    );
                    (None, true)
                }
            };

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
                listener,
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
                        match meta_rx.recv_timeout(Duration::from_millis(50)) {
                            Ok(msg) => {
                                let payload = RawEventPayload::Clipboard(RawClipboardEvent {
                                    format: msg.format,
                                    byte_length: msg.byte_length,
                                    hash_sha256: msg.hash_sha256,
                                    source_hwnd: msg.source_hwnd,
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
                                    EventSource::ClipboardListener,
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
        let (out_tx, out_rx) = bounded(5_000);
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
            listener: None,
            is_mock: true,
        }
    }

    pub fn is_mock(&self) -> bool {
        self.is_mock
    }

    pub fn receiver(&self) -> Receiver<RawEvent> {
        self.output_rx.clone()
    }

    /// Simulate clipboard copy event with safe SHA-256 hash digest (no raw content stored).
    pub fn simulate_copy(&self, format: &str, data: &[u8], source_hwnd: Option<u64>) {
        let hash = compute_sha256(data);
        let payload = RawEventPayload::Clipboard(RawClipboardEvent {
            format: format.to_string(),
            byte_length: data.len(),
            hash_sha256: hash,
            source_hwnd,
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
            EventSource::ClipboardListener,
            seq,
            payload,
        );
        let _ = self.output_tx.try_send(raw_event);
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        #[cfg(windows)]
        if let Some(mut listener) = self.listener.take() {
            listener.stop();
        }
        if let Some(h) = self.worker_handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for ClipboardManager {
    fn drop(&mut self) {
        self.stop();
    }
}
