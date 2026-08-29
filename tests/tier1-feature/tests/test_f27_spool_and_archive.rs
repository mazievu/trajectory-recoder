use archive::{chunk_and_encrypt_archive, create_tar_zstd_archive};
use spool::{SpoolDirectoryManager, SpoolState};
use tempfile::tempdir;

#[test]
fn test_f27_spool_pipeline_and_archive_chunking() {
    let dir = tempdir().unwrap();
    let mgr = SpoolDirectoryManager::new(dir.path()).unwrap();

    let sid = "SESS_E2E_01";
    let rec_path = mgr.session_path(SpoolState::Recording, sid);
    std::fs::create_dir_all(&rec_path).unwrap();
    std::fs::write(rec_path.join("events.raw.ndjson"), "{\"event\":1}\n{\"event\":2}\n").unwrap();
    std::fs::write(rec_path.join("session.db"), "MOCK_SQLITE_DATA").unwrap();

    // Transition recording -> finalizing
    let fin_path = mgr.transition(sid, SpoolState::Recording, SpoolState::Finalizing).unwrap();

    // Package into TAR.Zstd
    let staging_dir = dir.path().join("staging");
    let archive_path = staging_dir.join("session.tar.zst");
    let chunks_dir = staging_dir.join("chunks");

    let (uncompressed, _compressed, files) = create_tar_zstd_archive(&fin_path, &archive_path, 3).unwrap();
    assert_eq!(files.len(), 2);

    let manifest = chunk_and_encrypt_archive(
        &archive_path,
        &chunks_dir,
        sid,
        1024 * 1024,
        None,
        uncompressed,
        files,
    )
    .unwrap();

    assert!(manifest.chunk_count >= 1);
    assert_eq!(manifest.session_id, sid);

    // Transition finalizing -> pending_upload -> uploading -> uploaded
    let _ = mgr.transition(sid, SpoolState::Finalizing, SpoolState::PendingUpload).unwrap();
    let _ = mgr.transition(sid, SpoolState::PendingUpload, SpoolState::Uploading).unwrap();
    let _ = mgr.transition(sid, SpoolState::Uploading, SpoolState::Uploaded).unwrap();

    let uploaded = mgr.list_sessions(SpoolState::Uploaded).unwrap();
    assert_eq!(uploaded, vec![sid.to_string()]);
}
