use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use server::{
    create_jwt, create_router, verify_jwt, AppState, Claims,
    HeartbeatRequest, InitiateRequest,
};
use sha2::{Digest, Sha256};
use tower::ServiceExt;

#[tokio::test]
async fn test_jwt_adversarial_validation() {
    let secret = "server_jwt_test_secret_key_888";

    // 1. Valid token
    let valid_token = create_jwt("MACHINE_VALID", secret).unwrap();
    let claims = verify_jwt(&valid_token, secret).unwrap();
    assert_eq!(claims.sub, "MACHINE_VALID");

    // 2. Tampered / wrong secret
    let err_wrong_secret = verify_jwt(&valid_token, "wrong_secret_1234567890123456");
    assert!(err_wrong_secret.is_err());

    // 3. Malformed garbage string
    let err_garbage = verify_jwt("not.a.valid.jwt.token", secret);
    assert!(err_garbage.is_err());

    // 4. Expired token
    let expired_claims = Claims {
        sub: "MACHINE_EXPIRED".to_string(),
        exp: (Utc::now().timestamp() - 3600) as usize, // 1 hour in the past
        iat: (Utc::now().timestamp() - 7200) as usize,
        iss: "trajectory-server".to_string(),
    };
    let expired_token = encode(
        &Header::default(),
        &expired_claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap();

    let err_expired = verify_jwt(&expired_token, secret);
    assert!(err_expired.is_err(), "Expired JWT must be rejected");

    // 5. Wrong issuer
    let bad_issuer_claims = Claims {
        sub: "MACHINE_BAD_ISS".to_string(),
        exp: (Utc::now().timestamp() + 3600) as usize,
        iat: Utc::now().timestamp() as usize,
        iss: "malicious-issuer".to_string(),
    };
    let bad_issuer_token = encode(
        &Header::default(),
        &bad_issuer_claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap();

    let err_issuer = verify_jwt(&bad_issuer_token, secret);
    assert!(err_issuer.is_err(), "JWT with invalid issuer must be rejected");
}

#[tokio::test]
async fn test_heartbeat_mismatched_machine_identity_forbidden() {
    let state = AppState::new_in_memory();
    let app = create_router(state.clone());

    // Create a valid token for MACHINE_A
    let token_machine_a = create_jwt("MACHINE_A", &state.jwt_secret).unwrap();

    // Send heartbeat claiming to be MACHINE_B using MACHINE_A token
    let heartbeat_payload = HeartbeatRequest {
        machine_id: "MACHINE_B".to_string(),
        disk_usage_pct: 42.0,
        active_session_id: None,
    };

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/machines/heartbeat")
                .header("Authorization", format!("Bearer {}", token_machine_a))
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&heartbeat_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "Spoofed machine_id in heartbeat must return 403 Forbidden"
    );
}

#[tokio::test]
async fn test_corrupted_chunk_hashes_rejected() {
    let state = AppState::new_in_memory();
    let app = create_router(state);

    let session_id = "SESS_CORRUPT_01";
    let init_req = InitiateRequest {
        session_id: session_id.to_string(),
        chunk_count: 2,
        total_size_bytes: 200,
        archive_sha256: "some_sha".to_string(),
        machine_id: Some("M01".to_string()),
        schema_version: Some("1.0".to_string()),
        user_id: Some("U01".to_string()),
    };

    let init_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/sessions")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&init_req).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(init_resp.status(), StatusCode::OK);

    let chunk_data = b"Legitimate chunk data here";
    let real_sha = hex::encode(Sha256::digest(chunk_data));
    let corrupted_sha = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef";

    // 1. Send chunk with mismatched SHA-256 header
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/sessions/{}/chunks/0", session_id))
                .header("X-Chunk-SHA256", corrupted_sha)
                .body(Body::from(chunk_data.to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "Corrupted chunk SHA-256 header must return 400 Bad Request"
    );

    // 2. Send chunk with bit-flipped payload against real_sha
    let mut bit_flipped = chunk_data.to_vec();
    bit_flipped[0] ^= 0xFF;

    let resp_flipped = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/sessions/{}/chunks/0", session_id))
                .header("X-Chunk-SHA256", &real_sha)
                .body(Body::from(bit_flipped))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        resp_flipped.status(),
        StatusCode::BAD_REQUEST,
        "Bit-flipped payload with mismatched hash must return 400 Bad Request"
    );
}

#[tokio::test]
async fn test_out_of_order_chunks_upload_and_complete_reassembly() {
    let state = AppState::new_in_memory();
    let app = create_router(state);

    let session_id = "SESS_OUT_OF_ORDER_01";

    // Generate 6 chunks of varying content
    let chunks = vec![
        b"Chunk 0: Header and initial metadata\n".to_vec(),
        b"Chunk 1: Mouse movement trajectories\n".to_vec(),
        b"Chunk 2: Keypress and typing bursts\n".to_vec(),
        b"Chunk 3: Window focus transitions\n".to_vec(),
        b"Chunk 4: Screenshot WebP keyframes\n".to_vec(),
        b"Chunk 5: Finalized session footer\n".to_vec(),
    ];

    let total_chunks = chunks.len();
    let mut full_archive = Vec::new();
    for c in &chunks {
        full_archive.extend_from_slice(c);
    }
    let expected_archive_sha256 = hex::encode(Sha256::digest(&full_archive));
    let total_size = full_archive.len() as u64;

    // 1. Initiate session
    let init_req = InitiateRequest {
        session_id: session_id.to_string(),
        chunk_count: total_chunks,
        total_size_bytes: total_size,
        archive_sha256: expected_archive_sha256.clone(),
        machine_id: Some("M_OUT_ORDER".to_string()),
        schema_version: Some("1.0".to_string()),
        user_id: Some("U01".to_string()),
    };

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/sessions")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&init_req).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 2. Upload chunks in scrambled order: [4, 1, 5, 0, 3, 2]
    let upload_order = vec![4, 1, 5, 0, 3, 2];

    for (step, &chunk_idx) in upload_order.iter().enumerate() {
        let chunk_data = &chunks[chunk_idx];
        let chunk_sha = hex::encode(Sha256::digest(chunk_data));

        let put_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/v1/sessions/{}/chunks/{}", session_id, chunk_idx))
                    .header("X-Chunk-SHA256", &chunk_sha)
                    .body(Body::from(chunk_data.clone()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(put_resp.status(), StatusCode::OK);

        // Query upload status mid-way
        let status_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/v1/sessions/{}/upload-status", session_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(status_resp.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(status_resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let status_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        let uploaded = status_json["uploaded_chunks"].as_array().unwrap();
        assert_eq!(uploaded.len(), step + 1);

        if step + 1 < total_chunks {
            assert_eq!(status_json["is_complete"], false);

            // Premature complete attempt must fail
            let complete_attempt = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(format!("/api/v1/sessions/{}/complete", session_id))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(
                complete_attempt.status(),
                StatusCode::BAD_REQUEST,
                "Premature completion with missing chunks must return 400"
            );
        } else {
            assert_eq!(status_json["is_complete"], true);
            let missing = status_json["missing_chunks"].as_array().unwrap();
            assert!(missing.is_empty());
        }
    }

    // 3. Final complete session call after all out-of-order chunks are uploaded
    let final_complete_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/sessions/{}/complete", session_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(final_complete_resp.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(final_complete_resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let complete_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(complete_json["status"], "SESSION_ACCEPTED");
    assert_eq!(complete_json["archive_sha256_verified"], true);
}

#[tokio::test]
async fn test_archive_checksum_mismatch_unprocessable_entity() {
    let state = AppState::new_in_memory();
    let app = create_router(state);

    let session_id = "SESS_ARCHIVE_MISMATCH_99";
    let chunk_0 = b"Actual archive content 0".to_vec();
    let chunk_1 = b"Actual archive content 1".to_vec();
    let chunk_0_sha = hex::encode(Sha256::digest(&chunk_0));
    let chunk_1_sha = hex::encode(Sha256::digest(&chunk_1));

    // Initiate with bogus archive SHA-256
    let init_req = InitiateRequest {
        session_id: session_id.to_string(),
        chunk_count: 2,
        total_size_bytes: (chunk_0.len() + chunk_1.len()) as u64,
        archive_sha256: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        machine_id: Some("M01".to_string()),
        schema_version: Some("1.0".to_string()),
        user_id: Some("U01".to_string()),
    };

    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/sessions")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&init_req).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Upload both chunks with valid individual chunk hashes
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/sessions/{}/chunks/0", session_id))
                .header("X-Chunk-SHA256", &chunk_0_sha)
                .body(Body::from(chunk_0))
                .unwrap(),
        )
        .await
        .unwrap();

    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/sessions/{}/chunks/1", session_id))
                .header("X-Chunk-SHA256", &chunk_1_sha)
                .body(Body::from(chunk_1))
                .unwrap(),
        )
        .await
        .unwrap();

    // Complete session -> should detect full archive mismatch and return 422
    let comp_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/sessions/{}/complete", session_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        comp_resp.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "Full archive checksum mismatch must return 422 Unprocessable Entity"
    );
}

#[tokio::test]
async fn test_nonexistent_session_operations_return_404() {
    let state = AppState::new_in_memory();
    let app = create_router(state);

    // 1. Chunk upload to nonexistent session
    let chunk_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/sessions/NON_EXISTENT/chunks/0")
                .body(Body::from(b"data".to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(chunk_resp.status(), StatusCode::NOT_FOUND);

    // 2. Status for nonexistent session
    let status_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/sessions/NON_EXISTENT/upload-status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(status_resp.status(), StatusCode::NOT_FOUND);

    // 3. Complete nonexistent session
    let complete_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/sessions/NON_EXISTENT/complete")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(complete_resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_duplicate_chunk_upload_idempotency() {
    let state = AppState::new_in_memory();
    let app = create_router(state);

    let session_id = "SESS_IDEMPOTENT_01";
    let chunk_data = b"Repeatable chunk content".to_vec();
    let chunk_sha = hex::encode(Sha256::digest(&chunk_data));

    let init_req = InitiateRequest {
        session_id: session_id.to_string(),
        chunk_count: 1,
        total_size_bytes: chunk_data.len() as u64,
        archive_sha256: chunk_sha.clone(),
        machine_id: Some("M01".to_string()),
        schema_version: Some("1.0".to_string()),
        user_id: Some("U01".to_string()),
    };

    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/sessions")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&init_req).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Upload chunk 0 first time
    let resp1 = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/sessions/{}/chunks/0", session_id))
                .header("X-Chunk-SHA256", &chunk_sha)
                .body(Body::from(chunk_data.clone()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp1.status(), StatusCode::OK);

    // Re-upload chunk 0 second time (simulating client retry after dropped ACK)
    let resp2 = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/sessions/{}/chunks/0", session_id))
                .header("X-Chunk-SHA256", &chunk_sha)
                .body(Body::from(chunk_data))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp2.status(), StatusCode::OK);

    // Complete session successfully
    let comp_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/sessions/{}/complete", session_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(comp_resp.status(), StatusCode::OK);
}
