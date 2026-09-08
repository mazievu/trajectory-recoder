//! 6-stage spool state machine, startup crash recovery scanner, and 4-tier disk protection.

pub mod state;
pub mod watermark;

pub use state::{SpoolDirectoryManager, SpoolState};
pub use watermark::{DiskWatermarkConfig, DiskWatermarkLevel, evaluate_disk_level};

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_spool_state_transitions() {
        let dir = tempdir().unwrap();
        let mgr = SpoolDirectoryManager::new(dir.path()).unwrap();

        let sid = "TEST_SESSION_100";
        let rec_path = mgr.session_path(SpoolState::Recording, sid);
        std::fs::create_dir_all(&rec_path).unwrap();
        std::fs::write(rec_path.join("data.txt"), "sample").unwrap();

        // 1. recording -> finalizing
        let fin_path = mgr
            .transition(sid, SpoolState::Recording, SpoolState::Finalizing)
            .unwrap();
        assert!(fin_path.exists());
        assert!(!rec_path.exists());

        // 2. finalizing -> pending_upload
        let pen_path = mgr
            .transition(sid, SpoolState::Finalizing, SpoolState::PendingUpload)
            .unwrap();
        assert!(pen_path.exists());

        // 3. pending_upload -> uploading
        let upl_path = mgr
            .transition(sid, SpoolState::PendingUpload, SpoolState::Uploading)
            .unwrap();
        assert!(upl_path.exists());

        // 4. uploading -> uploaded
        let done_path = mgr
            .transition(sid, SpoolState::Uploading, SpoolState::Uploaded)
            .unwrap();
        assert!(done_path.exists());

        // Verify list
        let uploaded_list = mgr.list_sessions(SpoolState::Uploaded).unwrap();
        assert_eq!(uploaded_list, vec![sid.to_string()]);
    }
}
