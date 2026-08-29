use crate::model::WindowState;
use crate::topology::MonitorTopology;
use core_types::event::{EventSource, RawEvent, RawEventPayload, RawWindowEvent};
use core_types::id::GlobalEventId;
use core_types::timestamp::DualTimestamp;
use crossbeam_channel::{Receiver, Sender, bounded};
use parking_lot::RwLock;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tracing::{info, warn};

pub struct WindowTracker {
    machine_id: String,
    windows_session_id: u32,
    user_id: String,
    event_seq: Arc<AtomicU64>,
    global_seq: Arc<AtomicU64>,
    topology: MonitorTopology,
    current_foreground: Arc<RwLock<Option<WindowState>>>,
    output_tx: Sender<RawEvent>,
    output_rx: Receiver<RawEvent>,
    running: Arc<AtomicBool>,
    worker_handle: Option<JoinHandle<()>>,
    #[cfg(windows)]
    win_hook: Option<crate::hook::windows_hook::WinEventHookThread>,
    is_mock: bool,
}

impl WindowTracker {
    /// Start tracking windows using SetWinEventHook on Windows, or mock if running non-interactively.
    pub fn start(
        machine_id: impl Into<String>,
        windows_session_id: u32,
        user_id: impl Into<String>,
    ) -> Result<Self, String> {
        let (out_tx, out_rx) = bounded(10_000);
        let machine_id = machine_id.into();
        let user_id = user_id.into();
        let running = Arc::new(AtomicBool::new(true));
        let topology = MonitorTopology::new();
        topology.refresh_from_system();

        #[cfg(windows)]
        {
            let (raw_tx, raw_rx) = bounded(5_000);
            let win_hook_res = crate::hook::windows_hook::WinEventHookThread::start(raw_tx);
            let (win_hook, is_mock) = match win_hook_res {
                Ok(h) => (Some(h), false),
                Err(err) => {
                    warn!(
                        "Could not start SetWinEventHook (fallback to simulation/mock): {}",
                        err
                    );
                    (None, true)
                }
            };

            let mut tracker = Self {
                machine_id,
                windows_session_id,
                user_id,
                event_seq: Arc::new(AtomicU64::new(1)),
                global_seq: Arc::new(AtomicU64::new(1)),
                topology,
                current_foreground: Arc::new(RwLock::new(None)),
                output_tx: out_tx.clone(),
                output_rx: out_rx,
                running: running.clone(),
                worker_handle: None,
                win_hook,
                is_mock,
            };

            if !is_mock {
                let topo = tracker.topology.clone();
                let cur_fg = tracker.current_foreground.clone();
                let m_id = tracker.machine_id.clone();
                let u_id = tracker.user_id.clone();
                let sess_id = tracker.windows_session_id;
                let ev_seq = tracker.event_seq.clone();
                let g_seq = tracker.global_seq.clone();
                let r_clone = running.clone();

                let worker = thread::spawn(move || {
                    while r_clone.load(Ordering::Relaxed) {
                        match raw_rx.recv_timeout(Duration::from_millis(50)) {
                            Ok(msg) => {
                                let event_type = match msg.event_type {
                                    0x0003 => "FOREGROUND",
                                    0x0016 => "MINIMIZE",
                                    0x0017 => "RESTORE",
                                    0x8000 => "OPEN",
                                    0x8001 => "CLOSE",
                                    0x800B => "MOVE",
                                    _ => "UNKNOWN",
                                };

                                if event_type == "UNKNOWN" {
                                    continue;
                                }

                                let is_fg = event_type == "FOREGROUND";
                                let bounds = crate::win_api::native::get_window_rect(msg.hwnd);
                                let mon_id = topo.find_monitor_for_rect(&bounds);
                                let state =
                                    crate::win_api::native::inspect_window(msg.hwnd, mon_id, is_fg);

                                if is_fg {
                                    *cur_fg.write() = Some(state.clone());
                                }

                                let payload = RawEventPayload::Window(RawWindowEvent {
                                    event_type: event_type.to_string(),
                                    hwnd: state.hwnd,
                                    pid: state.pid,
                                    process_name: state.process_name,
                                    window_title: state.title,
                                    bounds: state.bounds,
                                    monitor_id: state.monitor_id,
                                    dpi: state.dpi,
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
                                    EventSource::WinEvent,
                                    seq,
                                    payload,
                                );
                                let _ = out_tx.try_send(raw_event);
                            }
                            Err(_) => {}
                        }
                    }
                });
                tracker.worker_handle = Some(worker);
            }

            Ok(tracker)
        }

        #[cfg(not(windows))]
        {
            Ok(Self::start_mock(machine_id, windows_session_id, user_id))
        }
    }

    /// Start in explicit mock/simulation mode.
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
            topology: MonitorTopology::new(),
            current_foreground: Arc::new(RwLock::new(None)),
            output_tx: out_tx,
            output_rx: out_rx,
            running: Arc::new(AtomicBool::new(true)),
            worker_handle: None,
            #[cfg(windows)]
            win_hook: None,
            is_mock: true,
        }
    }

    pub fn is_mock(&self) -> bool {
        self.is_mock
    }

    pub fn topology(&self) -> &MonitorTopology {
        &self.topology
    }

    pub fn current_foreground(&self) -> Option<WindowState> {
        self.current_foreground.read().clone()
    }

    pub fn receiver(&self) -> Receiver<RawEvent> {
        self.output_rx.clone()
    }

    // --- Synthetic simulation methods for tests and CI ---

    pub fn simulate_foreground_window(&self, state: WindowState) {
        *self.current_foreground.write() = Some(state.clone());
        self.simulate_window_event("FOREGROUND", state);
    }

    pub fn simulate_window_event(&self, event_type: &str, state: WindowState) {
        let payload = RawEventPayload::Window(RawWindowEvent {
            event_type: event_type.to_string(),
            hwnd: state.hwnd,
            pid: state.pid,
            process_name: state.process_name,
            window_title: state.title,
            bounds: state.bounds,
            monitor_id: state.monitor_id,
            dpi: state.dpi,
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
            EventSource::WinEvent,
            seq,
            payload,
        );
        let _ = self.output_tx.try_send(raw_event);
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        #[cfg(windows)]
        if let Some(mut hook) = self.win_hook.take() {
            hook.stop();
        }
        if let Some(h) = self.worker_handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for WindowTracker {
    fn drop(&mut self) {
        self.stop();
    }
}
