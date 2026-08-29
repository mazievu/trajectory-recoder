use axum::body::Body;
use axum::http::{Request, StatusCode};
use server::{create_jwt, create_router, verify_jwt, AppState, InitiateRequest};
use sha2::{Digest, Sha256};
use tower::ServiceExt;

#[tokio::test]
async fn test_jwt_generation_and_validation() {
    let secret = "my_secure_jwt_secret_key_987654321";
    let token = create_jwt("MACHINE_99", secret).unwrap();
    assert!(!token.is_empty());

    let claims = verify_jwt(&token, secret).unwrap();
    assert_eq!(claims.sub, "MACHINE_99");
    assert_eq!(claims.iss, "trajectory-server");

    // Invalid secret verification fails
    let err = verify_jwt(&token, "wrong_secret_key_111111111111111");
    assert!(err.is_err());
}

#[tokio::test]
async fn test_server_chunk_checksum_mismatch_rejected() {
    let state = AppState::new_in_memory();
    let app = create_router(state);

    let init_req = InitiateRequest {
        session_id: "SESS_ERR_01".to_string(),
        chunk_count: 1,
        total_size_bytes: 100,
        archive_sha256: "some_sha".to_string(),
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

    // Send chunk with mismatched SHA-256 header
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/sessions/SESS_ERR_01/chunks/0")
                .header("X-Chunk-SHA256", "0000000000000000000000000000000000000000000000000000000000000000")
                .body(Body::from(b"chunk real data".to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_server_complete_archive_checksum_mismatch_rejected() {
    let state = AppState::new_in_memory();
    let app = create_router(state);

    let chunk0 = b"chunk_zero_data";
    let chunk0_sha = hex::encode(Sha256::digest(chunk0));

    // Claim archive sha is different
    let init_req = InitiateRequest {
        session_id: "SESS_MISMATCH_01".to_string(),
        chunk_count: 1,
        total_size_bytes: chunk0.len() as u64,
        archive_sha256: "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_string(),
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

    // Upload chunk 0
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/sessions/SESS_MISMATCH_01/chunks/0")
                .header("X-Chunk-SHA256", &chunk0_sha)
                .body(Body::from(chunk0.to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Complete session -> should return 422 Unprocessable Entity
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/sessions/SESS_MISMATCH_01/complete")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_server_complete_missing_chunks_rejected() {
    let state = AppState::new_in_memory();
    let app = create_router(state);

    let init_req = InitiateRequest {
        session_id: "SESS_MISSING_01".to_string(),
        chunk_count: 3,
        total_size_bytes: 300,
        archive_sha256: "some_sha".to_string(),
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

    // Try complete before uploading chunks
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/sessions/SESS_MISSING_01/complete")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
