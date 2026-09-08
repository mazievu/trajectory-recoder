use spool::{
    DiskWatermarkConfig, DiskWatermarkLevel, SpoolDirectoryManager, SpoolState, evaluate_disk_level,
};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_spool_complete_lifecycle_transitions() {
    let dir = tempdir().expect("tempdir");
    let mgr = SpoolDirectoryManager::new(dir.path()).expect("spool manager");

    let sid = "SESSION_20260829_050000_UUID1234";

    // Create session in recording
    let rec_path = mgr.session_path(SpoolState::Recording, sid);
    fs::create_dir_all(rec_path.join("screenshots")).unwrap();
    fs::create_dir_all(rec_path.join("video")).unwrap();
    fs::write(rec_path.join("manifest.json"), "{\"status\":\"RECORDING\"}").unwrap();
    fs::write(rec_path.join("events.raw.ndjson"), "{\"e\":1}\n").unwrap();
    fs::write(rec_path.join("screenshots/shot_001.webp"), b"RIFF...WEBP").unwrap();

    // Stage 1: recording -> finalizing
    let fin_path = mgr
        .transition(sid, SpoolState::Recording, SpoolState::Finalizing)
        .unwrap();
    assert!(fin_path.exists());
    assert!(!rec_path.exists());
    assert!(fin_path.join("screenshots/shot_001.webp").exists());

    // Stage 2: finalizing -> pending_upload
    let pen_path = mgr
        .transition(sid, SpoolState::Finalizing, SpoolState::PendingUpload)
        .unwrap();
    assert!(pen_path.exists());
    assert!(!fin_path.exists());

    // Stage 3: pending_upload -> uploading
    let upl_path = mgr
        .transition(sid, SpoolState::PendingUpload, SpoolState::Uploading)
        .unwrap();
    assert!(upl_path.exists());
    assert!(!pen_path.exists());

    // Stage 4: uploading -> uploaded
    let upld_path = mgr
        .transition(sid, SpoolState::Uploading, SpoolState::Uploaded)
        .unwrap();
    assert!(upld_path.exists());
    assert!(!upl_path.exists());

    // Verify session listing in uploaded
    let uploaded = mgr.list_sessions(SpoolState::Uploaded).unwrap();
    assert_eq!(uploaded, vec![sid.to_string()]);
}

#[test]
fn test_spool_exposes_all_stage_directories_from_its_base_path() {
    let dir = tempdir().expect("tempdir");
    let mgr = SpoolDirectoryManager::new(dir.path()).expect("spool manager");

    assert_eq!(mgr.base_path(), dir.path());
    assert_eq!(mgr.recording_dir(), dir.path().join("recording"));
    assert_eq!(mgr.finalizing_dir(), dir.path().join("finalizing"));
    assert_eq!(mgr.pending_upload_dir(), dir.path().join("pending_upload"));
    assert_eq!(mgr.uploading_dir(), dir.path().join("uploading"));
    assert_eq!(mgr.uploaded_dir(), dir.path().join("uploaded"));
    assert_eq!(mgr.failed_dir(), dir.path().join("failed"));
}

#[test]
fn test_spool_error_transitions() {
    let dir = tempdir().expect("tempdir");
    let mgr = SpoolDirectoryManager::new(dir.path()).expect("spool manager");

    // Transition non-existent session
    let res = mgr.transition(
        "NON_EXISTENT_SESSION",
        SpoolState::Recording,
        SpoolState::Finalizing,
    );
    assert!(res.is_err());
    assert_eq!(res.unwrap_err().kind(), std::io::ErrorKind::NotFound);
}

#[test]
fn test_spool_purge_uploaded_chronological_ordering() {
    let dir = tempdir().expect("tempdir");
    let mgr = SpoolDirectoryManager::new(dir.path()).expect("spool manager");

    // Create 5 uploaded sessions
    let sids = [
        "SESS_20260829_010000",
        "SESS_20260829_020000",
        "SESS_20260829_030000",
        "SESS_20260829_040000",
        "SESS_20260829_050000",
    ];

    for sid in &sids {
        let p = mgr.session_path(SpoolState::Uploaded, sid);
        fs::create_dir_all(&p).unwrap();
        fs::write(p.join("dummy.bin"), b"123").unwrap();
    }

    assert_eq!(mgr.list_sessions(SpoolState::Uploaded).unwrap().len(), 5);

    // Keep max 2 -> should purge the 3 oldest (010000, 020000, 030000)
    let purged = mgr.purge_uploaded_older_than(2).unwrap();
    assert_eq!(purged, 3);

    let remaining = mgr.list_sessions(SpoolState::Uploaded).unwrap();
    assert_eq!(remaining.len(), 2);
    assert!(remaining.contains(&"SESS_20260829_040000".to_string()));
    assert!(remaining.contains(&"SESS_20260829_050000".to_string()));
}

#[test]
fn test_disk_watermarks_exhaustive_boundary_precision() {
    let config = DiskWatermarkConfig {
        low_water_percent: 70.0,
        high_water_percent: 85.0,
        critical_percent: 92.0,
    };

    let total = 10_000_000u64; // 10 MB total

    // 0.0% usage -> Normal (10,000,000 available)
    assert_eq!(
        evaluate_disk_level(total, 10_000_000, &config),
        DiskWatermarkLevel::Normal
    );

    // 69.999% usage -> Normal (3,000,100 available)
    assert_eq!(
        evaluate_disk_level(total, 3_000_100, &config),
        DiskWatermarkLevel::Normal
    );

    // 70.000% usage -> LowWater (3,000,000 available)
    assert_eq!(
        evaluate_disk_level(total, 3_000_000, &config),
        DiskWatermarkLevel::LowWater
    );

    // 84.999% usage -> LowWater (1,500,100 available)
    assert_eq!(
        evaluate_disk_level(total, 1_500_100, &config),
        DiskWatermarkLevel::LowWater
    );

    // 85.000% usage -> HighWater (1,500,000 available)
    assert_eq!(
        evaluate_disk_level(total, 1_500_000, &config),
        DiskWatermarkLevel::HighWater
    );

    // 91.999% usage -> HighWater (800,100 available)
    assert_eq!(
        evaluate_disk_level(total, 800_100, &config),
        DiskWatermarkLevel::HighWater
    );

    // 92.000% usage -> Critical (800,000 available)
    assert_eq!(
        evaluate_disk_level(total, 800_000, &config),
        DiskWatermarkLevel::Critical
    );

    // 99.999% usage -> Critical (100 available)
    assert_eq!(
        evaluate_disk_level(total, 100, &config),
        DiskWatermarkLevel::Critical
    );

    // 100.000% usage -> Critical (0 available)
    assert_eq!(
        evaluate_disk_level(total, 0, &config),
        DiskWatermarkLevel::Critical
    );

    // Division by zero guard: total_bytes == 0
    assert_eq!(
        evaluate_disk_level(0, 0, &config),
        DiskWatermarkLevel::Normal
    );

    // Overflow guard: available > total
    assert_eq!(
        evaluate_disk_level(total, total + 1000, &config),
        DiskWatermarkLevel::Normal
    );
}
