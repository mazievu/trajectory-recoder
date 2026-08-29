use crate::coordinate::{CoordinateMapper, MonitorBounds};
use crate::double_click::DoubleClickDetector;
use crate::keyboard_state::KeyboardModifierTracker;
use core_types::event::{
    EventSource, RawEvent, RawEventPayload, RawKeyboardEvent, RawMouseEvent,
};
use core_types::id::GlobalEventId;
use core_types::metadata::MouseButton;
use core_types::timestamp::DualTimestamp;
use crossbeam_channel::{bounded, Receiver, Sender};
use parking_lot::RwLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct InputEventEnvelope {
    pub raw_event: RawEvent,
}

/// Main manager for input capture via Win32 low-level hooks or simulation.
pub struct InputHookManager {
    machine_id: String,
    windows_session_id: u32,
    user_id: String,
    event_seq: Arc<AtomicU64>,
    global_seq: Arc<AtomicU64>,
    coordinate_mapper: CoordinateMapper,
    double_click_detector: Arc<RwLock<DoubleClickDetector>>,
    modifier_tracker: Arc<RwLock<KeyboardModifierTracker>>,
    output_tx: Sender<RawEvent>,
    output_rx: Receiver<RawEvent>,
    running: Arc<AtomicBool>,
    worker_handle: Option<JoinHandle<()>>,
    #[cfg(windows)]
    win_hook: Option<crate::hook::windows_hook::Win32HookThread>,
    is_mock: bool,
}

impl InputHookManager {
    /// Start with real Win32 hooks on Windows desktop, or mock if running non-interactively.
    pub fn start(
        machine_id: impl Into<String>,
        windows_session_id: u32,
        user_id: impl Into<String>,
    ) -> Result<Self, String> {
        let (out_tx, out_rx) = bounded(50_000);
        let machine_id = machine_id.into();
        let user_id = user_id.into();
        let running = Arc::new(AtomicBool::new(true));

        #[cfg(windows)]
        {
            let (mouse_tx, mouse_rx) = bounded(20_000);
            let (kbd_tx, kbd_rx) = bounded(20_000);

            let win_hook_result = crate::hook::windows_hook::Win32HookThread::start(mouse_tx.clone(), kbd_tx.clone());
            let (win_hook, is_mock) = match win_hook_result {
                Ok(hook) => (Some(hook), false),
                Err(err) => {
                    warn!("Could not initialize Win32 hooks (fallback to simulation/mock): {}", err);
                    (None, true)
                }
            };

            let mut mgr = Self {
                machine_id,
                windows_session_id,
                user_id,
                event_seq: Arc::new(AtomicU64::new(1)),
                global_seq: Arc::new(AtomicU64::new(1)),
                coordinate_mapper: CoordinateMapper::new(),
                double_click_detector: Arc::new(RwLock::new(DoubleClickDetector::default())),
                modifier_tracker: Arc::new(RwLock::new(KeyboardModifierTracker::default())),
                output_tx: out_tx.clone(),
                output_rx: out_rx,
                running: running.clone(),
                worker_handle: None,
                win_hook,
                is_mock,
            };

            if !is_mock {
                let coord_map = mgr.coordinate_mapper.clone();
                let dc_det = mgr.double_click_detector.clone();
                let mod_track = mgr.modifier_tracker.clone();
                let m_id = mgr.machine_id.clone();
                let u_id = mgr.user_id.clone();
                let sess_id = mgr.windows_session_id;
                let ev_seq = mgr.event_seq.clone();
                let g_seq = mgr.global_seq.clone();
                let r_clone = running.clone();

                let worker = thread::spawn(move || {
                    while r_clone.load(Ordering::Relaxed) {
                        crossbeam_channel::select! {
                            recv(mouse_rx) -> msg => {
                                if let Ok(m) = msg {
                                    let now_ms = SystemTime::now()
                                        .duration_since(UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_millis() as u64;

                                    let mut event_type = m.event_type.to_string();
                                    if m.event_type == "MOUSE_DOWN" {
                                        let is_double = dc_det.write().check_and_update(m.button, m.px, m.py, now_ms);
                                        if is_double {
                                            event_type = "DOUBLE_CLICK".to_string();
                                        }
                                    }

                                    let (mon_id, nx, ny, pt) = coord_map.map_point(m.px, m.py);
                                    let payload = RawEventPayload::Mouse(RawMouseEvent {
                                        event_type,
                                        button: m.button,
                                        coords: pt,
                                        monitor_id: mon_id,
                                        delta_x: m.delta_x,
                                        delta_y: m.delta_y,
                                        state: String::new(),
                                        physical_x: m.px,
                                        physical_y: m.py,
                                        normalized_x: nx,
                                        normalized_y: ny,
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
                                        EventSource::InputHook,
                                        seq,
                                        payload,
                                    );
                                    let _ = out_tx.try_send(raw_event);
                                }
                            },
                            recv(kbd_rx) -> msg => {
                                if let Ok(k) = msg {
                                    mod_track.write().update_vk(k.vk_code, k.is_down);
                                    let current_mods = mod_track.read().current_modifiers();
                                    let key_name = KeyboardModifierTracker::vk_to_key_name(k.vk_code);

                                    let payload = RawEventPayload::Keyboard(RawKeyboardEvent {
                                        event_type: k.event_type.to_string(),
                                        vk_code: k.vk_code,
                                        scan_code: k.scan_code,
                                        key_name,
                                        modifiers: current_mods,
                                        is_injected: k.is_injected,
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
                                        EventSource::InputHook,
                                        seq,
                                        payload,
                                    );
                                    let _ = out_tx.try_send(raw_event);
                                }
                            },
                            default(Duration::from_millis(50)) => {}
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

    /// Explicit mock/simulation mode.
    pub fn start_mock(
        machine_id: impl Into<String>,
        windows_session_id: u32,
        user_id: impl Into<String>,
    ) -> Self {
        let (out_tx, out_rx) = bounded(50_000);
        Self {
            machine_id: machine_id.into(),
            windows_session_id,
            user_id: user_id.into(),
            event_seq: Arc::new(AtomicU64::new(1)),
            global_seq: Arc::new(AtomicU64::new(1)),
            coordinate_mapper: CoordinateMapper::new(),
            double_click_detector: Arc::new(RwLock::new(DoubleClickDetector::default())),
            modifier_tracker: Arc::new(RwLock::new(KeyboardModifierTracker::default())),
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

    pub fn update_monitors(&self, monitors: Vec<MonitorBounds>) {
        self.coordinate_mapper.update_monitors(monitors);
    }

    /// Access the event receiver stream.
    pub fn receiver(&self) -> Receiver<RawEvent> {
        self.output_rx.clone()
    }

    // --- Synthetic event simulation for tests and CI ---

    pub fn simulate_mouse_move(&self, px: i32, py: i32) {
        let (mon_id, nx, ny, pt) = self.coordinate_mapper.map_point(px, py);
        let payload = RawEventPayload::Mouse(RawMouseEvent {
            event_type: "MOUSE_MOVE".into(),
            button: MouseButton::None,
            coords: pt,
            monitor_id: mon_id,
            delta_x: 0.0,
            delta_y: 0.0,
            state: String::new(),
            physical_x: px,
            physical_y: py,
            normalized_x: nx,
            normalized_y: ny,
        });
        self.emit_payload(payload);
    }

    pub fn simulate_mouse_down(&self, button: MouseButton, px: i32, py: i32) {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let is_double = self.double_click_detector.write().check_and_update(button, px, py, now_ms);
        let event_type = if is_double { "DOUBLE_CLICK" } else { "MOUSE_DOWN" };

        let (mon_id, nx, ny, pt) = self.coordinate_mapper.map_point(px, py);
        let payload = RawEventPayload::Mouse(RawMouseEvent {
            event_type: event_type.into(),
            button,
            coords: pt,
            monitor_id: mon_id,
            delta_x: 0.0,
            delta_y: 0.0,
            state: String::new(),
            physical_x: px,
            physical_y: py,
            normalized_x: nx,
            normalized_y: ny,
        });
        self.emit_payload(payload);
    }

    pub fn simulate_mouse_up(&self, button: MouseButton, px: i32, py: i32) {
        let (mon_id, nx, ny, pt) = self.coordinate_mapper.map_point(px, py);
        let payload = RawEventPayload::Mouse(RawMouseEvent {
            event_type: "MOUSE_UP".into(),
            button,
            coords: pt,
            monitor_id: mon_id,
            delta_x: 0.0,
            delta_y: 0.0,
            state: String::new(),
            physical_x: px,
            physical_y: py,
            normalized_x: nx,
            normalized_y: ny,
        });
        self.emit_payload(payload);
    }

    pub fn simulate_mouse_wheel(&self, px: i32, py: i32, delta_x: f64, delta_y: f64) {
        let (mon_id, nx, ny, pt) = self.coordinate_mapper.map_point(px, py);
        let payload = RawEventPayload::Mouse(RawMouseEvent {
            event_type: "MOUSE_WHEEL".into(),
            button: MouseButton::None,
            coords: pt,
            monitor_id: mon_id,
            delta_x,
            delta_y,
            state: String::new(),
            physical_x: px,
            physical_y: py,
            normalized_x: nx,
            normalized_y: ny,
        });
        self.emit_payload(payload);
    }

    pub fn simulate_key_down(&self, vk_code: u32, scan_code: u32) {
        self.modifier_tracker.write().update_vk(vk_code, true);
        let mods = self.modifier_tracker.read().current_modifiers();
        let key_name = KeyboardModifierTracker::vk_to_key_name(vk_code);

        let payload = RawEventPayload::Keyboard(RawKeyboardEvent {
            event_type: "KEY_DOWN".into(),
            vk_code,
            scan_code,
            key_name,
            modifiers: mods,
            is_injected: false,
        });
        self.emit_payload(payload);
    }

    pub fn simulate_key_up(&self, vk_code: u32, scan_code: u32) {
        self.modifier_tracker.write().update_vk(vk_code, false);
        let mods = self.modifier_tracker.read().current_modifiers();
        let key_name = KeyboardModifierTracker::vk_to_key_name(vk_code);

        let payload = RawEventPayload::Keyboard(RawKeyboardEvent {
            event_type: "KEY_UP".into(),
            vk_code,
            scan_code,
            key_name,
            modifiers: mods,
            is_injected: false,
        });
        self.emit_payload(payload);
    }

    fn emit_payload(&self, payload: RawEventPayload) {
        let seq = self.event_seq.fetch_add(1, Ordering::Relaxed);
        let gseq = self.global_seq.fetch_add(1, Ordering::Relaxed);
        let raw_event = RawEvent::new(
            seq,
            GlobalEventId::new(gseq),
            DualTimestamp::now(),
            self.machine_id.clone(),
            self.windows_session_id,
            self.user_id.clone(),
            EventSource::InputHook,
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

impl Drop for InputHookManager {
    fn drop(&mut self) {
        self.stop();
    }
}
