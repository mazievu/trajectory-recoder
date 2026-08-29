use clipboard_win::ClipboardManager;
use core_types::event::RawEventPayload;
use core_types::metadata::{BoundingRect, MouseButton};
use event_bus::{EventBus, EventBusConfig, Priority, PublishResult};
use file_events_win::FileWatcherManager;
use input_win::InputHookManager;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use window_win::{WindowState, WindowTracker};

#[test]
fn test_end_to_end_capture_core_event_bus_flow() {
    // 1. Initialize Event Bus
    let bus = Arc::new(EventBus::new(EventBusConfig::default()));
    let publisher = bus.publisher();
    let receiver = bus.receiver();

    // 2. Initialize Capture Core Managers (Simulation / Mock mode for clean deterministic E2E test)
    let input_mgr = InputHookManager::start_mock("TEST_PC_01", 1, "test_engineer");
    let window_mgr = WindowTracker::start_mock("TEST_PC_01", 1, "test_engineer");
    let clip_mgr = ClipboardManager::start_mock("TEST_PC_01", 1, "test_engineer");
    let file_mgr = FileWatcherManager::start_mock("TEST_PC_01", 1, "test_engineer");

    let running = Arc::new(AtomicBool::new(true));

    // 3. Connect managers to Event Bus via forwarder threads
    let forwarder_handles = vec![
        {
            let rx = input_mgr.receiver();
            let pub_handle = publisher.clone();
            let r = running.clone();
            thread::spawn(move || {
                while r.load(Ordering::Relaxed) {
                    if let Ok(ev) = rx.recv_timeout(Duration::from_millis(20)) {
                        let _ = pub_handle.publish_event(ev);
                    }
                }
            })
        },
        {
            let rx = window_mgr.receiver();
            let pub_handle = publisher.clone();
            let r = running.clone();
            thread::spawn(move || {
                while r.load(Ordering::Relaxed) {
                    if let Ok(ev) = rx.recv_timeout(Duration::from_millis(20)) {
                        let _ = pub_handle.publish_event(ev);
                    }
                }
            })
        },
        {
            let rx = clip_mgr.receiver();
            let pub_handle = publisher.clone();
            let r = running.clone();
            thread::spawn(move || {
                while r.load(Ordering::Relaxed) {
                    if let Ok(ev) = rx.recv_timeout(Duration::from_millis(20)) {
                        let _ = pub_handle.publish_event(ev);
                    }
                }
            })
        },
        {
            let rx = file_mgr.receiver();
            let pub_handle = publisher.clone();
            let r = running.clone();
            thread::spawn(move || {
                while r.load(Ordering::Relaxed) {
                    if let Ok(ev) = rx.recv_timeout(Duration::from_millis(20)) {
                        let _ = pub_handle.publish_event(ev);
                    }
                }
            })
        },
    ];

    // 4. Emit simulated events across all capture modalities
    // A. Active window foreground change (P1)
    let win_state = WindowState {
        hwnd: 0xDEADBEEF,
        pid: 8888,
        process_name: "Code.exe".into(),
        exe_path: "C:\\Program Files\\Microsoft VS Code\\Code.exe".into(),
        title: "trajectory-recorder - Visual Studio Code".into(),
        bounds: BoundingRect::new(0, 0, 1920, 1080),
        monitor_id: 0,
        dpi: 96,
        is_minimized: false,
        is_maximized: true,
        is_foreground: true,
    };
    window_mgr.simulate_foreground_window(win_state);

    // B. Mouse interaction: Move, Down, Up (P0)
    input_mgr.simulate_mouse_move(500, 300);
    input_mgr.simulate_mouse_down(MouseButton::Left, 500, 300);
    input_mgr.simulate_mouse_up(MouseButton::Left, 500, 300);

    // C. Keyboard typing: 'T', 'R' (P0)
    input_mgr.simulate_key_down(0x54, 0x14); // 'T'
    input_mgr.simulate_key_up(0x54, 0x14);
    input_mgr.simulate_key_down(0x52, 0x13); // 'R'
    input_mgr.simulate_key_up(0x52, 0x13);

    // D. Clipboard copy event (P1)
    clip_mgr.simulate_copy("CF_UNICODETEXT", b"Trajectory Capture", Some(0xDEADBEEF));

    // E. File save event (P1)
    file_mgr.simulate_file_event(
        "CREATED",
        "C:\\Users\\test_engineer\\Documents\\trajectory.json",
        None,
    );

    // 5. Drain and verify all events from the Event Bus
    let mut received_events = Vec::new();
    let start = std::time::Instant::now();
    let expected_count = 9; // 1 window + 3 mouse + 4 keyboard + 1 clipboard + 1 file = 10 total (or 9-10)

    while received_events.len() < 10 && start.elapsed() < Duration::from_secs(3) {
        if let Ok((priority, event)) = receiver.recv_timeout(Duration::from_millis(100)) {
            received_events.push((priority, event));
        }
    }

    assert!(
        received_events.len() >= 9,
        "Expected at least 9 events, received {}",
        received_events.len()
    );

    // Verify presence and integrity of each event type
    let mut has_window = false;
    let mut has_mouse = false;
    let mut has_keyboard = false;
    let mut has_clipboard = false;
    let mut has_file = false;

    for (p, ev) in &received_events {
        assert_eq!(ev.schema, "gtf.trajectory");
        assert_eq!(ev.schema_version, "1.0");
        assert_eq!(ev.machine_id, "TEST_PC_01");
        assert_eq!(ev.user_id, "test_engineer");

        match &ev.payload {
            RawEventPayload::Window(w) => {
                assert_eq!(*p, Priority::P1_Window);
                assert_eq!(w.process_name, "Code.exe");
                has_window = true;
            }
            RawEventPayload::Mouse(m) => {
                assert_eq!(*p, Priority::P0_Input);
                assert_eq!(m.monitor_id, 0);
                has_mouse = true;
            }
            RawEventPayload::Keyboard(k) => {
                assert_eq!(*p, Priority::P0_Input);
                assert!(k.key_name == "T" || k.key_name == "R");
                has_keyboard = true;
            }
            RawEventPayload::Clipboard(c) => {
                assert_eq!(*p, Priority::P1_Window);
                assert_eq!(c.format, "CF_UNICODETEXT");
                assert_eq!(c.byte_length, 18);
                assert_eq!(c.hash_sha256.len(), 64);
                has_clipboard = true;
            }
            RawEventPayload::File(f) => {
                assert_eq!(*p, Priority::P1_Window);
                assert_eq!(f.action, "CREATED");
                assert_eq!(f.file_path, "C:\\Users\\test_engineer\\Documents\\trajectory.json");
                has_file = true;
            }
            _ => {}
        }
    }

    assert!(has_window, "Missing window event");
    assert!(has_mouse, "Missing mouse event");
    assert!(has_keyboard, "Missing keyboard event");
    assert!(has_clipboard, "Missing clipboard event");
    assert!(has_file, "Missing file event");

    // Clean up
    running.store(false, Ordering::SeqCst);
    for h in forwarder_handles {
        let _ = h.join();
    }

    let metrics = bus.metrics();
    assert!(metrics.total_published >= 9);
    assert!(metrics.total_consumed >= 9);
}
