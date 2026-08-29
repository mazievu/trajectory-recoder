use e2e_runner::mock_server::start_mock_server;
use e2e_runner::verifiers::upload_verifier::UploadVerifier;

#[tokio::test]
async fn test_mock_ingestion_server_upload_lifecycle() {
    let server = start_mock_server().await.unwrap();
    let base_url = server.url();

    let dummy_payload = vec![42u8; 128 * 1024]; // 128 KiB test payload
    let session_id = "machine01_20260829_040000_abcd1234";

    let plan = UploadVerifier::execute_upload(&base_url, session_id, &dummy_payload, 32 * 1024)
        .await
        .unwrap();

    assert_eq!(plan.chunks.len(), 4);
    assert!(plan.chunks.iter().all(|c| c.is_uploaded));

    // Verify status on server
    {
        let sessions = server.state.sessions.lock().unwrap();
        let session = sessions.get(session_id).unwrap();
        assert!(session.is_completed);
        assert_eq!(session.uploaded_chunks.len(), 4);
    }

    server.stop();
}
