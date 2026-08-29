use archive::{chunk_and_encrypt_archive, create_tar_zstd_archive};
use server::{create_router, AppState};
use spool::{SpoolDirectoryManager, SpoolState};
use tempfile::tempdir;
use tokio::net::TcpListener;
use upload_client::{InitiateSessionRequest, UploadClient};

async fn start_server() -> (String, AppState) {
    let state = AppState::new_in_memory();
    let app = create_router(state.clone());

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (format!("http://{}", addr), state)
}

#[tokio::test]
async fn test_uploader_end_to_end_pipeline() {
    let (server_url, server_state) = start_server().await;
    let client = UploadClient::new(&server_url);

    let dir = tempdir().unwrap();
    let spool_mgr = SpoolDirectoryManager::new(dir.path()).unwrap();

    let sid = "SESS_E2E_001";
    let pending_dir = spool_mgr.session_path(SpoolState::PendingUpload, sid);
    tokio::fs::create_dir_all(&pending_dir).await.unwrap();

    // Create session content
    tokio::fs::write(pending_dir.join("events.raw.ndjson"), "{\"event\":\"mouse_down\"}\n{\"event\":\"key_press\"}\n").await.unwrap();
    tokio::fs::write(pending_dir.join("session.db"), b"SQLITE_DUMMY_DB_DATA").await.unwrap();

    // 1. Packaging & Chunking
    let staging_dir = pending_dir.join("_packaging");
    let archive_file = staging_dir.join("session.tar.zst");
    let chunks_dir = staging_dir.join("chunks");

    let (uncompressed, _compressed, file_list) = create_tar_zstd_archive(&pending_dir, &archive_file, 3).unwrap();
    let chunk_size = 64 * 1024; // 64KB chunks
    let manifest = chunk_and_encrypt_archive(
        &archive_file,
        &chunks_dir,
        sid,
        chunk_size,
        None,
        uncompressed,
        file_list,
    ).unwrap();

    assert!(manifest.chunk_count >= 1);

    // 2. Transition PendingUpload -> Uploading
    let uploading_dir = spool_mgr.transition(sid, SpoolState::PendingUpload, SpoolState::Uploading).unwrap();
    assert!(uploading_dir.exists());

    // 3. Initiate session on server
    let init_req = InitiateSessionRequest {
        session_id: sid.to_string(),
        chunk_count: manifest.chunk_count,
        total_size_bytes: manifest.compressed_size_bytes,
        archive_sha256: manifest.archive_sha256.clone(),
        machine_id: Some("MACH_TEST".to_string()),
        schema_version: Some("1.0".to_string()),
        user_id: Some("USER_TEST".to_string()),
    };
    let init_resp = client.initiate_session(&init_req).await.unwrap();
    assert_eq!(init_resp.status, "initiated");

    // 4. Upload chunks
    let chunks_upload_dir = uploading_dir.join("_packaging/chunks");
    for chunk_entry in &manifest.chunks {
        let chunk_path = chunks_upload_dir.join(&chunk_entry.file_name);
        client.upload_chunk_with_retry(sid, chunk_entry.chunk_index, &chunk_path, &chunk_entry.sha256).await.unwrap();
    }

    // 5. Complete session
    let complete_resp = client.complete_session(sid).await.unwrap();
    assert_eq!(complete_resp.status, "SESSION_ACCEPTED");
    assert!(complete_resp.archive_sha256_verified);

    // 6. Transition Uploading -> Uploaded
    let uploaded_dir = spool_mgr.transition(sid, SpoolState::Uploading, SpoolState::Uploaded).unwrap();
    assert!(uploaded_dir.exists());

    // Verify session state in server memory / object store
    let mem = server_state.mem_state.read();
    let sess_meta = mem.sessions.get(sid).unwrap();
    assert!(sess_meta.is_completed);
    assert_eq!(sess_meta.received_chunks.len(), manifest.chunk_count);
}
