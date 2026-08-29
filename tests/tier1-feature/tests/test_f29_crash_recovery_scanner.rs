use session::{repair_ndjson_tail, scan_and_recover_orphaned_sessions};
use tempfile::tempdir;

#[test]
fn test_f29_startup_crash_recovery_scan() {
    let dir = tempdir().unwrap();
    let recording_dir = dir.path().join("recording");
    let sess_dir = recording_dir.join("CRASHED_SESS_01");
    std::fs::create_dir_all(&sess_dir).unwrap();

    // Write partial corrupt NDJSON file
    let ndjson_path = sess_dir.join("events.raw.ndjson");
    let content = b"{\"event\": 1, \"type\": \"CLICK\"}\n{\"event\": 2, \"type\": \"KEY\"}\n{\"event\": 3, \"corrupted_unclosed";
    std::fs::write(&ndjson_path, content).unwrap();

    let recovered = scan_and_recover_orphaned_sessions(&recording_dir);
    assert_eq!(recovered.len(), 1);
    assert_eq!(recovered[0].session_id, "CRASHED_SESS_01");
    assert_eq!(recovered[0].recovered_events, 2);
    assert!(recovered[0].bytes_truncated > 0);

    // Verify repaired file contents
    let repaired_text = std::fs::read_to_string(&ndjson_path).unwrap();
    assert_eq!(
        repaired_text,
        "{\"event\": 1, \"type\": \"CLICK\"}\n{\"event\": 2, \"type\": \"KEY\"}\n"
    );
}
