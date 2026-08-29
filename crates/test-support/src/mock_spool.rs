use std::path::PathBuf;
use tempfile::TempDir;

pub struct MockSpoolFixture {
    pub root_dir: TempDir,
    pub recording_dir: PathBuf,
    pub finalizing_dir: PathBuf,
    pub pending_upload_dir: PathBuf,
    pub uploading_dir: PathBuf,
    pub uploaded_dir: PathBuf,
    pub failed_dir: PathBuf,
}

impl MockSpoolFixture {
    pub fn create() -> Self {
        let root = TempDir::new().expect("Failed to create tempdir for spool fixture");
        let path = root.path();

        let recording = path.join("recording");
        let finalizing = path.join("finalizing");
        let pending = path.join("pending_upload");
        let uploading = path.join("uploading");
        let uploaded = path.join("uploaded");
        let failed = path.join("failed");

        std::fs::create_dir_all(&recording).unwrap();
        std::fs::create_dir_all(&finalizing).unwrap();
        std::fs::create_dir_all(&pending).unwrap();
        std::fs::create_dir_all(&uploading).unwrap();
        std::fs::create_dir_all(&uploaded).unwrap();
        std::fs::create_dir_all(&failed).unwrap();

        Self {
            root_dir: root,
            recording_dir: recording,
            finalizing_dir: finalizing,
            pending_upload_dir: pending,
            uploading_dir: uploading,
            uploaded_dir: uploaded,
            failed_dir: failed,
        }
    }

    pub fn populate_mock_recording_session(
        &self,
        session_id: &str,
        valid_lines: usize,
        add_corrupt_tail: bool,
    ) -> PathBuf {
        let session_dir = self.recording_dir.join(session_id);
        std::fs::create_dir_all(&session_dir).unwrap();

        let raw_ndjson_path = session_dir.join("events.raw.ndjson");
        let mut content = String::new();
        for i in 0..valid_lines {
            content.push_str(&format!(
                r#"{{"schema":"gtf.trajectory","schema_version":"1.0","event_id":{},"payload":{{"type":"mouse_move"}}}}"#,
                i + 1
            ));
            content.push('\n');
        }

        if add_corrupt_tail {
            content.push_str(r#"{"schema":"gtf.trajectory","schema_version":"1.0","event_id":999,"payload":{"type":"partial_incomp"#);
        }

        std::fs::write(&raw_ndjson_path, content).unwrap();

        let manifest_path = session_dir.join("manifest.json");
        let manifest_content = format!(
            r#"{{"session_id":"{}","schema_version":"1.0","status":"RECORDING"}}"#,
            session_id
        );
        std::fs::write(manifest_path, manifest_content).unwrap();

        session_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_spool_fixture_creation() {
        let spool = MockSpoolFixture::create();
        assert!(spool.recording_dir.exists());
        assert!(spool.uploaded_dir.exists());

        let s_dir = spool.populate_mock_recording_session("sess_001", 10, true);
        assert!(s_dir.exists());
        assert!(s_dir.join("events.raw.ndjson").exists());
        assert!(s_dir.join("manifest.json").exists());
    }
}
