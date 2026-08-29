use e2e_runner::agent_controller::SpoolDirectoryManager;
use tempfile::TempDir;

#[test]
fn test_f22_spool_state_machine_directories() {
    let tmp = TempDir::new().unwrap();
    let spool = SpoolDirectoryManager::new(tmp.path().join("spool")).unwrap();

    assert!(spool.recording_dir().exists());
    assert!(spool.finalizing_dir().exists());
    assert!(spool.pending_upload_dir().exists());
    assert!(spool.uploaded_dir().exists());
    assert!(spool.failed_dir().exists());
}
