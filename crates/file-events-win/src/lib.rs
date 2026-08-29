//! Directory changes watcher and Common File Dialog hooks on Windows.

pub mod dialog;
pub mod filter;
pub mod manager;
pub mod watcher;

pub use dialog::FileDialogEvent;
pub use filter::is_noise_file;
pub use manager::FileWatcherManager;

#[cfg(test)]
mod tests {
    use super::*;
    use core_types::event::RawEventPayload;
    use std::time::Duration;

    #[test]
    fn test_noise_file_filtering() {
        assert!(is_noise_file("C:\\Users\\admin\\desktop.ini"));
        assert!(is_noise_file("C:\\Users\\admin\\Thumbs.db"));
        assert!(is_noise_file("C:\\Users\\admin\\~$QuarterlyReport.docx"));
        assert!(is_noise_file("C:\\Users\\admin\\data.tmp"));
        assert!(is_noise_file("C:\\Users\\admin\\download.crdownload"));

        assert!(!is_noise_file("C:\\Users\\admin\\Documents\\Report.docx"));
        assert!(!is_noise_file("C:\\Users\\admin\\Desktop\\Project.rs"));
    }

    #[test]
    fn test_file_watcher_simulation_pipeline() {
        let mgr = FileWatcherManager::start_mock("test_pc", 1, "test_user");
        let rx = mgr.receiver();

        // 1. Valid file create
        mgr.simulate_file_event("CREATED", "C:\\Users\\admin\\Documents\\new_doc.txt", None);

        // 2. Noise file (should be filtered out)
        mgr.simulate_file_event("CREATED", "C:\\Users\\admin\\Documents\\desktop.ini", None);

        // 3. Rename file
        mgr.simulate_file_event(
            "RENAMED",
            "C:\\Users\\admin\\Documents\\final_doc.txt",
            Some("C:\\Users\\admin\\Documents\\new_doc.txt".into()),
        );

        let e1 = rx.recv_timeout(Duration::from_millis(100)).unwrap();
        if let RawEventPayload::File(f) = e1.payload {
            assert_eq!(f.action, "CREATED");
            assert_eq!(f.file_path, "C:\\Users\\admin\\Documents\\new_doc.txt");
            assert!(f.old_file_path.is_none());
        } else {
            panic!("Expected File payload");
        }

        let e2 = rx.recv_timeout(Duration::from_millis(100)).unwrap();
        if let RawEventPayload::File(f) = e2.payload {
            assert_eq!(f.action, "RENAMED");
            assert_eq!(f.file_path, "C:\\Users\\admin\\Documents\\final_doc.txt");
            assert_eq!(
                f.old_file_path,
                Some("C:\\Users\\admin\\Documents\\new_doc.txt".into())
            );
        } else {
            panic!("Expected File payload");
        }

        // Noise file should not have produced an event
        assert!(rx.recv_timeout(Duration::from_millis(50)).is_err());
    }
}
