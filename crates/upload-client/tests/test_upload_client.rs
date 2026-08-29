use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post, put},
    Json, Router,
};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tempfile::tempdir;
use tokio::net::TcpListener;
use upload_client::{
    HeartbeatRequest, InitiateSessionRequest, RegisterMachineRequest, UploadClient,
    UploadClientConfig, UploadError,
};

#[derive(Default, Clone)]
struct TestServerState {
    received_chunks: Arc<Mutex<HashMap<usize, Vec<u8>>>>,
    attempt_counter: Arc<AtomicUsize>,
    fail_chunk_attempts: usize,
}

async fn start_test_server(fail_first_n_attempts: usize) -> (String, TestServerState) {
    let state = TestServerState {
        received_chunks: Arc::new(Mutex::new(HashMap::new())),
        attempt_counter: Arc::new(AtomicUsize::new(0)),
        fail_chunk_attempts: fail_first_n_attempts,
    };

    let app = Router::new()
        .route(
            "/api/v1/machines/register",
            post(|Json(_payload): Json<serde_json::Value>| async {
                (
                    StatusCode::OK,
                    Json(serde_json::json!({
                        "status": "registered",
                        "device_jwt": "jwt_mock_token_123",
                        "machine_id": "M01"
                    })),
                )
            }),
        )
        .route(
            "/api/v1/machines/heartbeat",
            post(|_headers: HeaderMap, Json(_payload): Json<serde_json::Value>| async {
                (StatusCode::OK, Json(serde_json::json!({ "status": "ok" })))
            }),
        )
        .route(
            "/api/v1/sessions",
            post(|Json(payload): Json<serde_json::Value>| async move {
                let sid = payload["session_id"].as_str().unwrap();
                (
                    StatusCode::OK,
                    Json(serde_json::json!({
                        "session_id": sid,
                        "upload_id": "up_123",
                        "status": "initiated"
                    })),
                )
            }),
        )
        .route(
            "/api/v1/sessions/:session_id/chunks/:chunk_index",
            put(
                |State(st): State<TestServerState>,
                 Path((_sid, idx)): Path<(String, usize)>,
                 headers: HeaderMap,
                 body: Bytes| async move {
                    let attempts = st.attempt_counter.fetch_add(1, Ordering::SeqCst);
                    if attempts < st.fail_chunk_attempts {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(serde_json::json!({ "error": "Transient error" })),
                        );
                    }

                    let claimed_sha = headers.get("X-Chunk-SHA256").unwrap().to_str().unwrap();
                    let actual_sha = hex::encode(Sha256::digest(&body));
                    if claimed_sha != actual_sha {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(serde_json::json!({ "error": "checksum mismatch" })),
                        );
                    }

                    st.received_chunks.lock().unwrap().insert(idx, body.to_vec());
                    (
                        StatusCode::OK,
                        Json(serde_json::json!({
                            "chunk_index": idx,
                            "status": "stored",
                            "sha256": actual_sha
                        })),
                    )
                },
            ),
        )
        .route(
            "/api/v1/sessions/:session_id/upload-status",
            get(|Path(sid): Path<String>| async move {
                (
                    StatusCode::OK,
                    Json(serde_json::json!({
                        "session_id": sid,
                        "uploaded_chunks": [0],
                        "missing_chunks": [],
                        "is_complete": true,
                        "status": "completed"
                    })),
                )
            }),
        )
        .route(
            "/api/v1/sessions/:session_id/complete",
            post(|Path(sid): Path<String>| async move {
                (
                    StatusCode::OK,
                    Json(serde_json::json!({
                        "status": "SESSION_ACCEPTED",
                        "session_id": sid,
                        "archive_sha256_verified": true
                    })),
                )
            }),
        )
        .with_state(state.clone());

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (format!("http://{}", addr), state)
}

#[tokio::test]
async fn test_full_upload_client_flow_with_retry_and_jitter() {
    // Fail first 2 attempts to exercise exponential backoff + jitter
    let (server_url, server_state) = start_test_server(2).await;

    let mut config = UploadClientConfig::default();
    config.initial_retry_backoff_ms = 50;
    config.max_retry_backoff_ms = 200;
    config.max_retries = 5;

    let mut client = UploadClient::with_config(&server_url, config);

    // 1. Register machine
    let reg_res = client
        .register_machine(&RegisterMachineRequest {
            machine_id: "M01".to_string(),
            hostname: "Host01".to_string(),
            os_version: "Windows 11".to_string(),
            registration_token: "tok_123".to_string(),
        })
        .await
        .unwrap();

    assert_eq!(reg_res.device_jwt, "jwt_mock_token_123");
    client.set_device_token(reg_res.device_jwt);

    // 2. Heartbeat
    client
        .send_heartbeat(&HeartbeatRequest {
            machine_id: "M01".to_string(),
            disk_usage_pct: 35.0,
            active_session_id: None,
        })
        .await
        .unwrap();

    // 3. Initiate Session
    let chunk_data = b"Sample payload for trajectory chunk testing.";
    let chunk_sha256 = hex::encode(Sha256::digest(chunk_data));

    let init_res = client
        .initiate_session(&InitiateSessionRequest {
            session_id: "SESS_100".to_string(),
            chunk_count: 1,
            total_size_bytes: chunk_data.len() as u64,
            archive_sha256: chunk_sha256.clone(),
            machine_id: Some("M01".to_string()),
            schema_version: Some("1.0".to_string()),
            user_id: Some("U01".to_string()),
        })
        .await
        .unwrap();

    assert_eq!(init_res.status, "initiated");

    // 4. Upload chunk file with retry
    let tmp = tempdir().unwrap();
    let chunk_file = tmp.path().join("chunk_0000.bin");
    tokio::fs::write(&chunk_file, chunk_data).await.unwrap();

    client
        .upload_chunk_with_retry("SESS_100", 0, &chunk_file, &chunk_sha256)
        .await
        .expect("Chunk upload should succeed after retry");

    assert_eq!(server_state.attempt_counter.load(Ordering::SeqCst), 3);
    assert_eq!(
        server_state.received_chunks.lock().unwrap().get(&0).unwrap(),
        chunk_data
    );

    // 5. Get status
    let status_res = client.get_upload_status("SESS_100").await.unwrap();
    assert_eq!(status_res.uploaded_chunks, vec![0]);
    assert!(status_res.missing_chunks.is_empty());

    // 6. Complete session
    let comp_res = client.complete_session("SESS_100").await.unwrap();
    assert_eq!(comp_res.status, "SESSION_ACCEPTED");
    assert!(comp_res.archive_sha256_verified);
}

#[tokio::test]
async fn test_upload_client_checksum_mismatch_error() {
    let (server_url, _) = start_test_server(0).await;
    let client = UploadClient::new(&server_url);

    let tmp = tempdir().unwrap();
    let chunk_file = tmp.path().join("chunk_0000.bin");
    tokio::fs::write(&chunk_file, b"Data").await.unwrap();

    let err = client
        .upload_chunk_with_retry("SESS_101", 0, &chunk_file, "wrong_hash")
        .await
        .unwrap_err();

    match err {
        UploadError::ChecksumMismatch { expected, computed } => {
            assert_eq!(expected, "wrong_hash");
            assert_eq!(computed, hex::encode(Sha256::digest(b"Data")));
        }
        other => panic!("Expected ChecksumMismatch, got {:?}", other),
    }
}
