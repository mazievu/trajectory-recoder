use config::{ConfigManager, ConfigValidationError, RecorderConfig, Validate};
use core_types::{
    ActionParameters, ActionType, CanonicalAction, CanonicalActionBuilder, ClickParams,
    ClipboardParams, ContextMetadata, DialogParams, DragDropParams, DualTimestamp,
    FileOperationParams, GlobalEventId, KeyPressParams, ModifierState, MouseButton,
    NavigationParams, Point2D, RawEvent, RawEventPayload, ScrollDirection, ScrollParams,
    SessionEventId, SessionId, ShortcutParams, SystemStateParams, TargetMetadata, TypeTextParams,
    UnknownParams, WaitParams, WindowLifecycleParams, SCHEMA_IDENTIFIER, SCHEMA_VERSION,
};
use ipc::{IpcError, IpcMessage, MsgPackCodec, MAX_IPC_FRAME_SIZE};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use test_support::MockNamedPipePair;
use tokio_util::codec::{Decoder, Encoder};
use bytes::{BufMut, BytesMut};

// =========================================================================
// AREA 1: IPC ADVERSARIAL STRESS & CORRUPTION HARNESS
// =========================================================================

#[test]
fn test_ipc_burst_encode_decode_10k_messages() {
    let mut codec = MsgPackCodec::<IpcMessage>::default();
    let mut buffer = BytesMut::new();
    let message_count = 10_000;

    let sample_messages = vec![
        IpcMessage::CommandResumeCapture,
        IpcMessage::CommandPauseCapture {
            reason: "Disk quota reached".to_string(),
        },
        IpcMessage::SessionBoundarySignal {
            previous_session_id: "sess_prev_001".to_string(),
            new_session_id: "sess_next_002".to_string(),
            event_count: 50_000,
        },
        IpcMessage::DiskWatermarkAlert {
            disk_tier: 2,
            free_bytes: 5_000_000_000,
            total_bytes: 500_000_000_000,
        },
        IpcMessage::GetStatusRequest,
    ];

    // Encode burst of 10,000 messages into contiguous buffer
    for i in 0..message_count {
        let msg = &sample_messages[i % sample_messages.len()];
        codec.encode(msg.clone(), &mut buffer).expect("Encode failed");
    }

    assert!(buffer.len() > message_count * 5);

    // Decode and verify all 10,000 messages in FIFO order
    let mut decoded_count = 0;
    while let Some(decoded_msg) = codec.decode(&mut buffer).expect("Decode failed") {
        let expected_msg = &sample_messages[decoded_count % sample_messages.len()];
        assert_eq!(&decoded_msg, expected_msg);
        decoded_count += 1;
    }

    assert_eq!(decoded_count, message_count);
    assert_eq!(buffer.len(), 0);
}

#[test]
fn test_ipc_partial_frames_single_byte_fragmentation() {
    let mut codec = MsgPackCodec::<IpcMessage>::default();
    let mut encoded_buf = BytesMut::new();

    let msg = IpcMessage::ConfigUpdate {
        config_toml: "version = 2\n[capture]\nvideo_fps = 30".to_string(),
        version: 2,
    };
    codec.encode(msg.clone(), &mut encoded_buf).expect("Encode failed");

    let total_len = encoded_buf.len();
    let mut feed_buf = BytesMut::new();

    // Feed bytes one-by-one into the decoder
    for (i, &byte) in encoded_buf.iter().enumerate() {
        feed_buf.put_u8(byte);
        let res = codec.decode(&mut feed_buf).expect("Decode step should not error");
        if i + 1 < total_len {
            assert!(res.is_none(), "Premature decoding at byte {} of {}", i + 1, total_len);
        } else {
            assert_eq!(res, Some(msg.clone()), "Failed to decode full frame on final byte");
        }
    }

    assert_eq!(feed_buf.len(), 0);
}

#[test]
fn test_ipc_malformed_oversized_frame_rejection() {
    let mut codec = MsgPackCodec::<IpcMessage>::default();
    let mut buf = BytesMut::new();

    // Put a length prefix that exceeds MAX_IPC_FRAME_SIZE (64 MiB + 1)
    let huge_len = (MAX_IPC_FRAME_SIZE as u32) + 1;
    buf.put_u32(huge_len);
    buf.put_slice(&[0x90, 0x01, 0x02]); // Put some dummy bytes

    let result = codec.decode(&mut buf);
    match result {
        Err(IpcError::FrameTooLarge { size, max }) => {
            assert_eq!(size, huge_len as usize);
            assert_eq!(max, MAX_IPC_FRAME_SIZE);
        }
        other => panic!("Expected FrameTooLarge error, got: {:?}", other),
    }
}

#[test]
fn test_ipc_malformed_corrupt_payload_rejection() {
    let mut codec = MsgPackCodec::<IpcMessage>::default();
    let mut buf = BytesMut::new();

    // Payload length of 10 bytes, but invalid MessagePack binary junk
    let payload = [0xFF, 0xFF, 0x00, 0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE];
    buf.put_u32(payload.len() as u32);
    buf.put_slice(&payload);

    let result = codec.decode(&mut buf);
    match result {
        Err(IpcError::DeserializationError(msg)) => {
            assert!(!msg.is_empty());
        }
        other => panic!("Expected DeserializationError, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_ipc_duplex_disconnect_and_channel_closure_resilience() {
    // Test that channel closures or connection resets don't cause deadlocks
    let (tx, mut rx) = tokio::sync::mpsc::channel::<IpcMessage>(10);
    let cancel = tokio_util::sync::CancellationToken::new();

    let client = ipc::ReconnectingIpcClient::new(
        "\\\\.\\pipe\\nonexistent-pipe-for-test",
        rx,
        tx.clone(),
        cancel.clone(),
    );

    // Spawn client and immediately cancel
    let handle = tokio::spawn(client.run());
    cancel.cancel();

    // Must cleanly terminate in less than 500ms
    tokio::time::timeout(Duration::from_millis(500), handle)
        .await
        .expect("Client should terminate cleanly on cancellation")
        .expect("Task panicked");
}

// =========================================================================
// AREA 2: CONFIG CONCURRENCY & VALIDATION STRESS HARNESS
// =========================================================================

#[tokio::test]
async fn test_config_high_concurrency_reads_and_writes() {
    let initial_config = RecorderConfig::default();
    let config_mgr = Arc::new(ConfigManager::new(initial_config).unwrap());
    let is_running = Arc::new(AtomicBool::new(true));
    let read_counter = Arc::new(AtomicU64::new(0));

    let mut reader_handles = Vec::new();

    // Spawn 32 concurrent reader tasks
    for _ in 0..32 {
        let mgr = Arc::clone(&config_mgr);
        let running = Arc::clone(&is_running);
        let counter = Arc::clone(&read_counter);

        reader_handles.push(tokio::spawn(async move {
            while running.load(Ordering::Relaxed) {
                let cfg = mgr.get();
                // Validate that we never read torn or inconsistent state
                let fps = cfg.capture.video_fps;
                assert!(fps >= 1 && fps <= 60, "Corrupt video_fps: {fps}");
                let chunk_size = cfg.upload.chunk_size_mb;
                assert!(chunk_size >= 64 && chunk_size <= 256, "Corrupt chunk_size: {chunk_size}");
                counter.fetch_add(1, Ordering::Relaxed);
                tokio::task::yield_now().await;
            }
        }));
    }

    // Spawn 4 concurrent writer tasks mutating config
    let mut writer_handles = Vec::new();
    for writer_id in 0..4 {
        let mgr = Arc::clone(&config_mgr);
        writer_handles.push(tokio::spawn(async move {
            for i in 0..1000 {
                let mut new_cfg = (*mgr.get()).clone();
                new_cfg.capture.video_fps = (10 + (i % 50)) as u32;
                new_cfg.upload.chunk_size_mb = 64 + (i % 128);
                new_cfg.privacy.excluded_apps.push(format!("App_{writer_id}_{i}.exe"));
                mgr.update(new_cfg).expect("Config update failed");
                tokio::task::yield_now().await;
            }
        }));
    }

    // Wait for all writers to finish
    for h in writer_handles {
        h.await.unwrap();
    }

    is_running.store(false, Ordering::Relaxed);

    for h in reader_handles {
        h.await.unwrap();
    }

    let total_reads = read_counter.load(Ordering::Relaxed);
    assert!(total_reads > 10_000, "Expected >10k concurrent reads, got {}", total_reads);
}

#[test]
fn test_config_validation_adversarial_matrix() {
    // 1. Invalid version (0)
    let mut cfg = RecorderConfig::default();
    cfg.version = 0;
    assert_eq!(cfg.validate(), Err(ConfigValidationError::InvalidVersion(0)));

    // 2. Chunk size boundaries (<64 or >256)
    let mut cfg = RecorderConfig::default();
    cfg.upload.chunk_size_mb = 0;
    assert_eq!(cfg.validate(), Err(ConfigValidationError::InvalidChunkSize(0)));
    cfg.upload.chunk_size_mb = 63;
    assert_eq!(cfg.validate(), Err(ConfigValidationError::InvalidChunkSize(63)));
    cfg.upload.chunk_size_mb = 257;
    assert_eq!(cfg.validate(), Err(ConfigValidationError::InvalidChunkSize(257)));

    // 3. Screenshot quality (0 or >100)
    let mut cfg = RecorderConfig::default();
    cfg.capture.screenshot_quality = 0;
    assert_eq!(cfg.validate(), Err(ConfigValidationError::InvalidScreenshotQuality(0)));
    cfg.capture.screenshot_quality = 101;
    assert_eq!(cfg.validate(), Err(ConfigValidationError::InvalidScreenshotQuality(101)));

    // 4. Screenshot diff threshold (<0.0 or >1.0)
    let mut cfg = RecorderConfig::default();
    cfg.capture.screenshot_diff_threshold = -0.01;
    assert!(matches!(cfg.validate(), Err(ConfigValidationError::InvalidDiffThreshold(_))));
    cfg.capture.screenshot_diff_threshold = 1.01;
    assert!(matches!(cfg.validate(), Err(ConfigValidationError::InvalidDiffThreshold(_))));

    // 5. Video FPS (0 or >60)
    let mut cfg = RecorderConfig::default();
    cfg.capture.video_fps = 0;
    assert_eq!(cfg.validate(), Err(ConfigValidationError::InvalidVideoFps(0)));
    cfg.capture.video_fps = 61;
    assert_eq!(cfg.validate(), Err(ConfigValidationError::InvalidVideoFps(61)));

    // 6. Video Bitrate (<100 or >50000)
    let mut cfg = RecorderConfig::default();
    cfg.capture.video_bitrate_kbps = 99;
    assert_eq!(cfg.validate(), Err(ConfigValidationError::InvalidVideoBitrate(99)));
    cfg.capture.video_bitrate_kbps = 50001;
    assert_eq!(cfg.validate(), Err(ConfigValidationError::InvalidVideoBitrate(50001)));

    // 7. Entropy threshold (<0.0 or >8.0)
    let mut cfg = RecorderConfig::default();
    cfg.privacy.entropy_threshold = -0.5;
    assert!(matches!(cfg.validate(), Err(ConfigValidationError::InvalidEntropyThreshold(_))));
    cfg.privacy.entropy_threshold = 8.1;
    assert!(matches!(cfg.validate(), Err(ConfigValidationError::InvalidEntropyThreshold(_))));

    // 8. Malformed regex patterns
    let mut cfg = RecorderConfig::default();
    cfg.privacy.custom_regex_patterns = vec!["[a-z0-9(".to_string()];
    assert!(matches!(cfg.validate(), Err(ConfigValidationError::InvalidRegexPattern { .. })));

    // 9. Inverted disk pressure watermarks
    let mut cfg = RecorderConfig::default();
    cfg.spool.disk_pressure_level1_pct = 85;
    cfg.spool.disk_pressure_level2_pct = 70; // L1 > L2
    assert!(matches!(cfg.validate(), Err(ConfigValidationError::InvalidDiskThresholds { .. })));
    cfg.spool.disk_pressure_level1_pct = 70;
    cfg.spool.disk_pressure_level2_pct = 85;
    cfg.spool.disk_pressure_level3_pct = 101; // L3 > 100
    assert!(matches!(cfg.validate(), Err(ConfigValidationError::InvalidDiskThresholds { .. })));

    // 10. Retry backoff initial > max
    let mut cfg = RecorderConfig::default();
    cfg.upload.initial_retry_backoff_ms = 10_000;
    cfg.upload.max_retry_backoff_ms = 1_000;
    assert!(matches!(cfg.validate(), Err(ConfigValidationError::InvalidRetryBackoff { .. })));

    // 11. Empty server URL
    let mut cfg = RecorderConfig::default();
    cfg.upload.server_url = "   ".to_string();
    assert!(matches!(cfg.validate(), Err(ConfigValidationError::InvalidServerUrl(_))));

    // 12. Invalid Server port 0
    let mut cfg = RecorderConfig::default();
    cfg.server.http_port = 0;
    assert_eq!(cfg.validate(), Err(ConfigValidationError::InvalidHttpPort));
}

// =========================================================================
// AREA 3: CORE TYPES DUAL TIMESTAMPS & 39 ACTION TYPES SERDE HARNESS
// =========================================================================

#[test]
fn test_dual_timestamp_ordering_and_window_boundaries() {
    let base_wall = chrono::Utc::now();
    let t0 = DualTimestamp::from_parts(base_wall, 10_000_000, 0);
    let t1 = DualTimestamp::from_parts(base_wall, 10_500_000, 0); // +500 µs (0.5 ms)
    let t2 = DualTimestamp::from_parts(base_wall, 20_000_000, 0); // +10 ms

    // Monotonic comparison
    assert!(t1.monotonic_ns > t0.monotonic_ns);
    assert!(t2.monotonic_ns > t1.monotonic_ns);

    // Duration calculation
    assert_eq!(t1.duration_since(&t0), Some(Duration::from_nanos(500_000)));
    assert_eq!(t0.duration_since(&t1), None, "Reverse duration should return None");

    // Elapsed ms
    assert_eq!(t2.elapsed_ms_since(&t0), Some(10));
    assert_eq!(t1.elapsed_ms_since(&t0), Some(0)); // < 1ms truncates to 0ms

    // Window checks
    assert!(t1.is_within_window(&t0, Duration::from_micros(500)));
    assert!(t1.is_within_window(&t0, Duration::from_micros(501)));
    assert!(!t1.is_within_window(&t0, Duration::from_micros(499)));

    // Commutativity of is_within_window
    assert!(t0.is_within_window(&t1, Duration::from_micros(500)));
    assert!(!t0.is_within_window(&t1, Duration::from_micros(499)));
}

#[test]
fn test_all_39_action_types_and_parameters_roundtrip() {
    let all_39_actions = vec![
        (ActionType::Click, ActionParameters::Click(ClickParams::default())),
        (ActionType::DoubleClick, ActionParameters::DoubleClick(ClickParams::default())),
        (ActionType::RightClick, ActionParameters::RightClick(ClickParams::default())),
        (ActionType::MiddleClick, ActionParameters::MiddleClick(ClickParams::default())),
        (ActionType::DragDrop, ActionParameters::DragDrop(DragDropParams::default())),
        (ActionType::Scroll, ActionParameters::Scroll(ScrollParams::default())),
        (ActionType::TypeText, ActionParameters::TypeText(TypeTextParams::default())),
        (ActionType::KeyPress, ActionParameters::KeyPress(KeyPressParams::default())),
        (ActionType::Shortcut, ActionParameters::Shortcut(ShortcutParams::default())),
        (ActionType::Copy, ActionParameters::Clipboard(ClipboardParams::default())),
        (ActionType::Cut, ActionParameters::Clipboard(ClipboardParams::default())),
        (ActionType::Paste, ActionParameters::Clipboard(ClipboardParams::default())),
        (ActionType::WindowSwitch, ActionParameters::Window(WindowLifecycleParams::default())),
        (ActionType::WindowOpen, ActionParameters::Window(WindowLifecycleParams::default())),
        (ActionType::WindowClose, ActionParameters::Window(WindowLifecycleParams::default())),
        (ActionType::AppOpen, ActionParameters::Window(WindowLifecycleParams::default())),
        (ActionType::AppClose, ActionParameters::Window(WindowLifecycleParams::default())),
        (ActionType::Navigate, ActionParameters::Navigation(NavigationParams::default())),
        (ActionType::FileOpen, ActionParameters::File(FileOperationParams::default())),
        (ActionType::FileSave, ActionParameters::File(FileOperationParams::default())),
        (ActionType::FileSaveAs, ActionParameters::File(FileOperationParams::default())),
        (ActionType::FileCreate, ActionParameters::File(FileOperationParams::default())),
        (ActionType::FileCopy, ActionParameters::File(FileOperationParams::default())),
        (ActionType::FileMove, ActionParameters::File(FileOperationParams::default())),
        (ActionType::FileRename, ActionParameters::File(FileOperationParams::default())),
        (ActionType::FileDelete, ActionParameters::File(FileOperationParams::default())),
        (ActionType::FileUpload, ActionParameters::File(FileOperationParams::default())),
        (ActionType::FileDownload, ActionParameters::File(FileOperationParams::default())),
        (ActionType::FileExport, ActionParameters::File(FileOperationParams::default())),
        (ActionType::DialogOpen, ActionParameters::Dialog(DialogParams::default())),
        (ActionType::DialogConfirm, ActionParameters::Dialog(DialogParams::default())),
        (ActionType::DialogCancel, ActionParameters::Dialog(DialogParams::default())),
        (ActionType::Wait, ActionParameters::Wait(WaitParams::default())),
        (ActionType::UserIdle, ActionParameters::System(SystemStateParams::default())),
        (ActionType::SystemLock, ActionParameters::System(SystemStateParams::default())),
        (ActionType::SystemUnlock, ActionParameters::System(SystemStateParams::default())),
        (ActionType::SystemSleep, ActionParameters::System(SystemStateParams::default())),
        (ActionType::SystemResume, ActionParameters::System(SystemStateParams::default())),
        (ActionType::UnknownInteraction, ActionParameters::Unknown(UnknownParams::default())),
    ];

    assert_eq!(all_39_actions.len(), 39, "Must strictly verify exactly 39 ActionTypes");

    let base_ts = DualTimestamp::now();

    for (idx, (action_type, params)) in all_39_actions.into_iter().enumerate() {
        let action = CanonicalActionBuilder::new(
            GlobalEventId::new((idx + 1) as u64),
            SessionId::new("session_test_39_actions"),
            idx as u64,
            base_ts,
            action_type,
            params,
        )
        .confidence(0.95)
        .duration_ms(120)
        .build();

        // 1. JSON Roundtrip
        let json_str = serde_json::to_string(&action).expect("JSON serialize");
        let deserialized_json: CanonicalAction = serde_json::from_str(&json_str).expect("JSON deserialize");
        assert_eq!(action, deserialized_json);

        // 2. MessagePack Roundtrip
        let msgpack_bytes = rmp_serde::to_vec_named(&action).expect("MsgPack serialize");
        let deserialized_msgpack: CanonicalAction = rmp_serde::from_slice(&msgpack_bytes).expect("MsgPack deserialize");
        assert_eq!(action, deserialized_msgpack);
    }
}

#[tokio::test]
async fn test_multithreaded_timestamp_monotonicity_and_concurrency() {
    let thread_count = 16;
    let samples_per_thread = 5_000;
    let mut handles = Vec::new();

    for _ in 0..thread_count {
        handles.push(tokio::spawn(async move {
            let mut prev = DualTimestamp::now();
            for _ in 0..samples_per_thread {
                let curr = DualTimestamp::now();
                assert!(
                    curr.monotonic_ns >= prev.monotonic_ns,
                    "Monotonic clock went backwards! prev={}, curr={}",
                    prev.monotonic_ns,
                    curr.monotonic_ns
                );
                prev = curr;
            }
            prev
        }));
    }

    for h in handles {
        h.await.unwrap();
    }
}

#[test]
fn test_all_10_raw_event_payload_variants_roundtrip() {
    use core_types::*;

    let now = DualTimestamp::now();
    let sample_raw_events = vec![
        RawEvent::new(
            1,
            GlobalEventId::new(1),
            now,
            "mach_1".to_string(),
            1,
            "user_1".to_string(),
            EventSource::Win32Hook,
            1,
            RawEventPayload::Mouse(RawMouseEvent::default()),
        ),
        RawEvent::new(
            2,
            GlobalEventId::new(2),
            now,
            "mach_1".to_string(),
            1,
            "user_1".to_string(),
            EventSource::Win32Hook,
            2,
            RawEventPayload::Keyboard(RawKeyboardEvent::default()),
        ),
        RawEvent::new(
            3,
            GlobalEventId::new(3),
            now,
            "mach_1".to_string(),
            1,
            "user_1".to_string(),
            EventSource::WinEvent,
            3,
            RawEventPayload::Window(RawWindowEvent::default()),
        ),
        RawEvent::new(
            4,
            GlobalEventId::new(4),
            now,
            "mach_1".to_string(),
            1,
            "user_1".to_string(),
            EventSource::UiAutomation,
            4,
            RawEventPayload::UiAutomation(RawUiaEvent::default()),
        ),
        RawEvent::new(
            5,
            GlobalEventId::new(5),
            now,
            "mach_1".to_string(),
            1,
            "user_1".to_string(),
            EventSource::BrowserExtension,
            5,
            RawEventPayload::Browser(RawBrowserEvent::default()),
        ),
        RawEvent::new(
            6,
            GlobalEventId::new(6),
            now,
            "mach_1".to_string(),
            1,
            "user_1".to_string(),
            EventSource::ClipboardListener,
            6,
            RawEventPayload::Clipboard(RawClipboardEvent::default()),
        ),
        RawEvent::new(
            7,
            GlobalEventId::new(7),
            now,
            "mach_1".to_string(),
            1,
            "user_1".to_string(),
            EventSource::FileWatcher,
            7,
            RawEventPayload::File(RawFileEvent::default()),
        ),
        RawEvent::new(
            8,
            GlobalEventId::new(8),
            now,
            "mach_1".to_string(),
            1,
            "user_1".to_string(),
            EventSource::WgcScreenCapture,
            8,
            RawEventPayload::Screen(RawScreenEvent::default()),
        ),
        RawEvent::new(
            9,
            GlobalEventId::new(9),
            now,
            "mach_1".to_string(),
            1,
            "user_1".to_string(),
            EventSource::SystemTelemetry,
            9,
            RawEventPayload::System(RawSystemEvent::default()),
        ),
        RawEvent::new(
            10,
            GlobalEventId::new(10),
            now,
            "mach_1".to_string(),
            1,
            "user_1".to_string(),
            EventSource::SessionRouter,
            10,
            RawEventPayload::Session(RawSessionEvent::default()),
        ),
    ];

    assert_eq!(sample_raw_events.len(), 10);

    for ev in sample_raw_events {
        let json_str = serde_json::to_string(&ev).unwrap();
        let deserialized: RawEvent = serde_json::from_str(&json_str).unwrap();
        assert_eq!(ev, deserialized);
    }
}

#[test]
fn test_config_file_save_and_load_roundtrip() {
    let mut config = RecorderConfig::default();
    config.machine.machine_id = "TEST-PC-ROUNDTRIP".to_string();
    config.capture.video_fps = 25;
    config.privacy.excluded_apps.push("KeePass.exe".to_string());

    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("recorder_test.toml");

    let manager = ConfigManager::new(config.clone()).unwrap();
    manager.save_to_file(&config_path).unwrap();

    let loaded_manager = ConfigManager::from_file(&config_path).unwrap();
    assert_eq!(loaded_manager.get().machine.machine_id, "TEST-PC-ROUNDTRIP");
    assert_eq!(loaded_manager.get().capture.video_fps, 25);
    assert!(loaded_manager.get().privacy.excluded_apps.contains(&"KeePass.exe".to_string()));
}

