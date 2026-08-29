use config::ConfigManager;
use core_types::{
    ActionType, DualTimestamp, GlobalEventId, MouseButton, SCHEMA_IDENTIFIER, SCHEMA_VERSION,
};
use crypto::{MasterKey, XChaCha20Aead, compute_sha256_hex, verify_sha256_hex};
use diagnostics::{
    HealthProbe, HealthStatus, MetricsCollector, ProbeResult, SystemHealthAggregator,
};
use ipc::{IpcMessage, MsgPackCodec};
use std::collections::HashMap;
use std::time::Duration;
use test_support::{
    FakeInputDriver, MockEventGenerator, MockNamedPipePair, MockSpoolFixture, SyntheticUiaTree,
};
use tokio_util::codec::{Decoder, Encoder};

struct TestDiskProbe;

impl HealthProbe for TestDiskProbe {
    fn name(&self) -> &'static str {
        "disk-pressure"
    }

    fn check(&self) -> ProbeResult {
        ProbeResult {
            probe_name: "disk-pressure".to_string(),
            status: HealthStatus::Healthy,
            message: "Disk usage at 45%".to_string(),
            checked_at: chrono::Utc::now(),
            metadata: HashMap::new(),
        }
    }
}

#[tokio::test]
async fn test_phase1_foundation_full_integration() {
    // 1. Diagnostics Subsystem: Metrics & Health
    let metrics = MetricsCollector::new();
    let health_agg = SystemHealthAggregator::new("TEST-MACH-01", "test-agent");
    health_agg.register_probe(Box::new(TestDiskProbe));

    let health_report = health_agg.evaluate();
    assert_eq!(health_report.overall_status, HealthStatus::Healthy);
    assert_eq!(health_report.probe_results.len(), 1);

    // 2. Config Subsystem: Load & Policy Override
    let config_mgr = ConfigManager::new(config::RecorderConfig::default()).expect("Valid config");
    assert_eq!(config_mgr.get().capture.video_fps, 10);

    config_mgr
        .apply_server_policy_override(|cfg| {
            cfg.capture.video_fps = 15;
            cfg.privacy.excluded_apps.push("SecApp.exe".to_string());
        })
        .expect("Server override success");

    assert_eq!(config_mgr.get().capture.video_fps, 15);
    assert!(
        config_mgr
            .get()
            .privacy
            .excluded_apps
            .contains(&"SecApp.exe".to_string())
    );

    // 3. Test-Support Subsystem: Synthetic UIA & Mock Events
    let uia_tree = SyntheticUiaTree::new_standard_form();
    let target = uia_tree
        .query_element_at_point(100, 110, Duration::from_millis(50))
        .await
        .expect("Query success")
        .expect("Found element");

    assert_eq!(target.automation_id, Some("txt_username".to_string()));
    assert!(!target.is_password);

    let mut event_gen =
        MockEventGenerator::new(0x12345678, "session_20260829_090000_abcd", "TEST-MACH-01");
    let mut raw_events = Vec::new();
    for i in 0..100 {
        let raw_ev = event_gen.generate_mouse_click_raw(100 + i, 200 + i, MouseButton::Left);
        metrics.record_event_captured();
        raw_events.push(raw_ev);
    }

    assert_eq!(raw_events.len(), 100);
    assert_eq!(
        metrics
            .events_captured_total
            .load(std::sync::atomic::Ordering::Relaxed),
        100
    );

    let canonical_action = event_gen.generate_canonical_action(ActionType::Click, "SubmitButton");
    metrics.record_canonical_action();
    assert_eq!(canonical_action.schema, SCHEMA_IDENTIFIER);
    assert_eq!(canonical_action.schema_version, SCHEMA_VERSION);
    assert_eq!(canonical_action.action_type, ActionType::Click);

    // 4. Crypto Subsystem: Master Key, DPAPI, AEAD, SHA-256
    let master_key = MasterKey::generate();
    let raw_events_json = serde_json::to_vec(&raw_events).unwrap();
    let hash_hex = compute_sha256_hex(&raw_events_json);
    assert!(verify_sha256_hex(&hash_hex, &hash_hex));

    let aad = b"session_id=session_20260829_090000_abcd;chunk=0";
    let encrypted_chunk =
        XChaCha20Aead::encrypt(&master_key, &raw_events_json, aad).expect("Encryption");
    let decrypted_chunk =
        XChaCha20Aead::decrypt(&master_key, &encrypted_chunk, aad).expect("Decryption");
    assert_eq!(decrypted_chunk, raw_events_json);

    // 5. IPC Subsystem: MsgPack Codec Framing & Duplex Streams
    let mut mock_pipe = MockNamedPipePair::new(65536);
    let ipc_msg = IpcMessage::SessionBoundarySignal {
        previous_session_id: "prev_sess".to_string(),
        new_session_id: "new_sess".to_string(),
        event_count: 100,
    };

    MockNamedPipePair::write_message(&mut mock_pipe.client_stream, &ipc_msg)
        .await
        .expect("Write msg");
    let received_msg = MockNamedPipePair::read_message(&mut mock_pipe.server_stream)
        .await
        .expect("Read msg");
    assert_eq!(ipc_msg, received_msg);

    // 6. Test-Support Mock Spool Fixture
    let spool_fixture = MockSpoolFixture::create();
    let session_dir =
        spool_fixture.populate_mock_recording_session("session_20260829_090000_abcd", 50, false);
    assert!(session_dir.exists());
    assert!(session_dir.join("events.raw.ndjson").exists());
    assert!(session_dir.join("manifest.json").exists());

    // 7. Diagnostics Final Snapshot
    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.events_captured_total, 100);
    assert_eq!(snapshot.canonical_actions_total, 1);
}
