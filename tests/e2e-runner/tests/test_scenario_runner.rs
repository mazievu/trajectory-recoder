use e2e_runner::scenario::ScenarioRunner;

#[test]
fn test_scenario_runner_session_lifecycle_and_spool_transitions() {
    let runner = ScenarioRunner::new().unwrap();
    let session_id = "test_session_spool_01";

    let session_dir = runner.create_test_session(session_id).unwrap();
    assert!(session_dir.exists());

    // Write events
    let sample_events = vec![
        serde_json::json!({
            "global_event_id": 1,
            "event_type": "CLICK",
            "timestamp": { "wall_time_utc": "2026-08-29T03:00:00Z", "monotonic_ns": 1000000, "timezone_offset_secs": 0 }
        }),
        serde_json::json!({
            "global_event_id": 2,
            "event_type": "TYPE_TEXT",
            "timestamp": { "wall_time_utc": "2026-08-29T03:00:01Z", "monotonic_ns": 2000000, "timezone_offset_secs": 0 }
        }),
    ];
    ScenarioRunner::write_sample_ndjson_events(&session_dir, &sample_events).unwrap();

    // Transition recording -> finalizing
    let fin_dir = runner.spool.transition_recording_to_finalizing(session_id).unwrap();
    assert!(fin_dir.exists());
    assert!(!runner.spool.recording_dir().join(session_id).exists());

    // Transition finalizing -> pending_upload
    let pend_dir = runner.spool.transition_finalizing_to_pending(session_id).unwrap();
    assert!(pend_dir.exists());

    // Transition pending_upload -> uploaded
    let up_dir = runner.spool.transition_pending_to_uploaded(session_id).unwrap();
    assert!(up_dir.exists());
}

#[test]
fn test_scenario_runner_19_attribute_audit() {
    let canonical_actions = vec![
        serde_json::json!({ "action_type": "APP_OPEN" }),
        serde_json::json!({ "action_type": "WINDOW_SWITCH" }),
        serde_json::json!({ "action_type": "WINDOW_STATE" }),
        serde_json::json!({ "action_type": "CLICK" }),
        serde_json::json!({ "action_type": "TYPE_TEXT" }),
        serde_json::json!({ "action_type": "SHORTCUT" }),
        serde_json::json!({ "action_type": "COPY" }),
        serde_json::json!({ "action_type": "PASTE" }),
        serde_json::json!({ "action_type": "FILE_OPEN" }),
        serde_json::json!({ "action_type": "DIALOG_CONFIRM" }),
        serde_json::json!({ "action_type": "FILE_UPLOAD" }),
        serde_json::json!({ "action_type": "FILE_DOWNLOAD" }),
        serde_json::json!({ "action_type": "DRAG_DROP" }),
        serde_json::json!({ "action_type": "SCROLL" }),
        serde_json::json!({ "action_type": "DIALOG_OPEN" }),
        serde_json::json!({ "action_type": "DIALOG_ACTION" }),
        serde_json::json!({ "action_type": "WAIT" }),
        serde_json::json!({ "action_type": "STATE_CHANGE" }),
        serde_json::json!({ "action_type": "TERMINAL_STATE" }),
    ];

    let audit_report = ScenarioRunner::audit_19_attributes(&canonical_actions);
    assert_eq!(audit_report.total_attributes, 19);
    assert_eq!(audit_report.verified_attributes, 19);
    assert_eq!(audit_report.failed_attributes, 0);
    assert!(audit_report.is_100_percent_compliant);
}
