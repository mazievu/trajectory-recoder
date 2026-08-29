use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::put,
    Json, Router,
};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::net::TcpListener;
use upload_client::{UploadClient, UploadClientConfig, UploadError};

#[derive(Default, Clone)]
struct FlakyServerState {
    received_chunks: Arc<Mutex<HashMap<usize, Vec<u8>>>>,
    attempt_counters: Arc<Mutex<HashMap<usize, usize>>>,
    fail_until_attempt: usize,
    inject_status_code: StatusCode,
}

async fn start_flaky_server(fail_until_attempt: usize, inject_status: StatusCode) -> (String, FlakyServerState) {
    let state = FlakyServerState {
        received_chunks: Arc::new(Mutex::new(HashMap::new())),
        attempt_counters: Arc::new(Mutex::new(HashMap::new())),
        fail_until_attempt,
        inject_status_code: inject_status,
    };

    let app = Router::new()
        .route(
            "/api/v1/sessions/:session_id/chunks/:chunk_index",
            put(
                |State(st): State<FlakyServerState>,
                 Path((_sid, idx)): Path<(String, usize)>,
                 headers: HeaderMap,
                 body: Bytes| async move {
                    let mut counters = st.attempt_counters.lock().unwrap();
                    let count = counters.entry(idx).or_insert(0);
                    *count += 1;
                    let current_attempt = *count;
                    drop(counters);

                    if current_attempt <= st.fail_until_attempt {
                        return (
                            st.inject_status_code,
                            Json(serde_json::json!({ "error": "Simulated transient failure" })),
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
        .with_state(state.clone());

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (format!("http://{}", addr), state)
}

#[tokio::test]
async fn test_upload_retry_eventual_success_after_multiple_failures() {
    // Fail first 3 attempts with HTTP 503, succeed on attempt 4
    let (server_url, server_state) = start_flaky_server(3, StatusCode::SERVICE_UNAVAILABLE).await;

    let mut config = UploadClientConfig::default();
    config.initial_retry_backoff_ms = 20;
    config.max_retry_backoff_ms = 100;
    config.max_retries = 6;

    let client = UploadClient::with_config(&server_url, config);

    let chunk_data = b"Stress testing retry loop with transient 503s.".to_vec();
    let chunk_sha = hex::encode(Sha256::digest(&chunk_data));

    let res = client
        .upload_chunk_bytes_with_retry("SESS_RETRY_01", 0, chunk_data.clone(), &chunk_sha)
        .await;

    assert!(res.is_ok(), "Upload should succeed after retries");

    let attempts = *server_state.attempt_counters.lock().unwrap().get(&0).unwrap();
    assert_eq!(attempts, 4, "Should have succeeded on exactly the 4th attempt");

    let stored = server_state.received_chunks.lock().unwrap().get(&0).unwrap().clone();
    assert_eq!(stored, chunk_data);
}

#[tokio::test]
async fn test_upload_retry_exhaustion_returns_max_retries_exceeded() {
    // Fail first 10 attempts with HTTP 500, but client max_retries is 4
    let (server_url, server_state) = start_flaky_server(10, StatusCode::INTERNAL_SERVER_ERROR).await;

    let mut config = UploadClientConfig::default();
    config.initial_retry_backoff_ms = 10;
    config.max_retry_backoff_ms = 50;
    config.max_retries = 4;

    let client = UploadClient::with_config(&server_url, config);

    let chunk_data = b"Exhaustion test chunk".to_vec();
    let chunk_sha = hex::encode(Sha256::digest(&chunk_data));

    let res = client
        .upload_chunk_bytes_with_retry("SESS_EXHAUST_01", 0, chunk_data, &chunk_sha)
        .await;

    match res {
        Err(UploadError::MaxRetriesExceeded(attempts)) => {
            assert_eq!(attempts, 4);
        }
        other => panic!("Expected MaxRetriesExceeded(4), got {:?}", other),
    }

    let attempts = *server_state.attempt_counters.lock().unwrap().get(&0).unwrap();
    assert_eq!(attempts, 4, "Should have stopped attempting after 4 retries");
}

#[tokio::test]
async fn test_upload_backoff_timing_and_jitter_bounds() {
    // Fail first 3 attempts to measure backoff sleep
    // backoff progression: 50ms, 100ms, 200ms
    let (server_url, _) = start_flaky_server(3, StatusCode::BAD_GATEWAY).await;

    let mut config = UploadClientConfig::default();
    config.initial_retry_backoff_ms = 50;
    config.max_retry_backoff_ms = 500;
    config.max_retries = 5;

    let client = UploadClient::with_config(&server_url, config);

    let chunk_data = b"Backoff timing test data".to_vec();
    let chunk_sha = hex::encode(Sha256::digest(&chunk_data));

    let start = Instant::now();
    let res = client
        .upload_chunk_bytes_with_retry("SESS_TIMING_01", 0, chunk_data, &chunk_sha)
        .await;
    let elapsed = start.elapsed();

    assert!(res.is_ok());
    assert!(
        elapsed.as_millis() >= 300,
        "Elapsed time ({:?}) should reflect cumulative exponential backoff",
        elapsed
    );
}

#[tokio::test]
async fn test_upload_client_concurrent_chunk_uploads_with_retries() {
    // 10 concurrent chunk uploads where each chunk fails 2 times before succeeding
    let (server_url, server_state) = start_flaky_server(2, StatusCode::INTERNAL_SERVER_ERROR).await;

    let mut config = UploadClientConfig::default();
    config.initial_retry_backoff_ms = 15;
    config.max_retry_backoff_ms = 100;
    config.max_retries = 5;

    let client = UploadClient::with_config(&server_url, config);

    let num_chunks = 10;
    let mut tasks = Vec::new();

    for chunk_idx in 0..num_chunks {
        let client_clone = client.clone();
        let chunk_data = format!("Chunk data payload for index {}", chunk_idx).into_bytes();
        let chunk_sha = hex::encode(Sha256::digest(&chunk_data));

        tasks.push(tokio::spawn(async move {
            client_clone
                .upload_chunk_bytes_with_retry("SESS_CONCUR_01", chunk_idx, chunk_data, &chunk_sha)
                .await
        }));
    }

    for (idx, task) in tasks.into_iter().enumerate() {
        let result = task.await.unwrap();
        assert!(result.is_ok(), "Chunk {} upload failed: {:?}", idx, result);
    }

    // Verify all 10 chunks are stored on server
    let stored = server_state.received_chunks.lock().unwrap();
    assert_eq!(stored.len(), num_chunks);
    for idx in 0..num_chunks {
        assert!(stored.contains_key(&idx));
    }
}

#[tokio::test]
async fn test_upload_unauthorized_token_handling() {
    let app = Router::new().route(
        "/api/v1/sessions/:session_id/chunks/:chunk_index",
        put(|headers: HeaderMap| async move {
            let auth = headers.get("Authorization").and_then(|h| h.to_str().ok());
            if auth != Some("Bearer valid_secret_token") {
                return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({ "error": "Unauthorized" })));
            }
            (StatusCode::OK, Json(serde_json::json!({ "status": "stored" })))
        }),
    );

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let server_url = format!("http://{}", addr);

    // 1. Test with invalid token
    let mut client_bad = UploadClient::with_config(
        &server_url,
        UploadClientConfig {
            max_retries: 2,
            initial_retry_backoff_ms: 10,
            ..Default::default()
        },
    );
    client_bad.set_device_token("invalid_token");

    let chunk_data = b"Unauthorized test data".to_vec();
    let chunk_sha = hex::encode(Sha256::digest(&chunk_data));

    let res = client_bad
        .upload_chunk_bytes_with_retry("SESS_AUTH_01", 0, chunk_data.clone(), &chunk_sha)
        .await;

    // Retries should exhaust and fail
    assert!(res.is_err());

    // 2. Test with valid token
    let mut client_good = UploadClient::with_config(
        &server_url,
        UploadClientConfig {
            max_retries: 2,
            initial_retry_backoff_ms: 10,
            ..Default::default()
        },
    );
    client_good.set_device_token("valid_secret_token");

    let res_good = client_good
        .upload_chunk_bytes_with_retry("SESS_AUTH_01", 0, chunk_data, &chunk_sha)
        .await;

    assert!(res_good.is_ok());
}
