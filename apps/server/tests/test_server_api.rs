use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::{Duration, Utc};
use server::{
    AppState, HeartbeatRequest, InitiateRequest, ProductionConfig, RegisterRequest, create_jwt,
    create_router, validate_server_deployment, verify_jwt, verify_object_store_readiness,
};
use sha2::{Digest, Sha256};
use tower::ServiceExt;

#[test]
fn jwt_rejects_an_empty_signing_secret() {
    assert!(create_jwt("MACHINE_99", "").is_err());
    assert!(verify_jwt("not-a-token", "").is_err());
}

#[test]
fn production_config_rejects_insecure_secrets_and_http_storage() {
    let mut config = ProductionConfig {
        database_url: "postgres://localhost/trajectory".to_string(),
        jwt_secret: "a".repeat(32),
        enrollment_token: "b".repeat(16),
        dashboard_api_token: "c".repeat(32),
        dashboard_assets_dir: std::path::PathBuf::from("/opt/trajectory/dashboard"),
        s3_bucket: "trajectory-archives".to_string(),
        s3_region: "us-east-1".to_string(),
        s3_endpoint: "https://object.example.test".to_string(),
        s3_access_key: "access".to_string(),
        s3_secret_key: "secret".to_string(),
    };
    assert!(config.validate().is_ok());

    config.jwt_secret.clear();
    assert!(config.validate().is_err());
    config.jwt_secret = "a".repeat(32);
    config.s3_endpoint = "http://object.example.test".to_string();
    assert!(config.validate().is_err());
}

#[test]
fn server_deployment_role_rejects_clients_and_client_only_settings() {
    assert!(validate_server_deployment(Some("server"), &[]).is_ok());
    assert!(validate_server_deployment(None, &[]).is_err());
    assert!(validate_server_deployment(Some("client"), &[]).is_err());
    assert!(validate_server_deployment(Some("server"), &[("SERVER_URL", Some("https://x"))]).is_err());
    assert!(validate_server_deployment(
        Some("server"),
        &[("DEVICE_TOKEN", Some("device-token")), ("SPOOL_DIR", None)],
    )
    .is_err());
    assert!(
        validate_server_deployment(
            Some("server"),
            &[(
                "TRAJECTORY_SERVER_URL",
                Some("https://collector.example.test")
            )],
        )
        .is_err()
    );
    assert!(
        validate_server_deployment(
            Some("server"),
            &[(
                "TRAJECTORY_ENROLLMENT_TOKEN",
                Some("client-enrollment-token")
            )],
        )
        .is_err()
    );
}

#[tokio::test]
async fn object_store_readiness_probe_requires_a_reachable_store() {
    let store = object_store::memory::InMemory::new();
    verify_object_store_readiness(&store)
        .await
        .expect("an available object store must pass the startup readiness probe");
}

#[tokio::test]
async fn registration_rejects_an_invalid_enrollment_token() {
    let app = create_router(AppState::new_in_memory());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/machines/register")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&RegisterRequest {
                        machine_id: "MACHINE_01".to_string(),
                        hostname: "host".to_string(),
                        os_version: "Windows".to_string(),
                        registration_token: "wrong-token".to_string(),
                    })
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn dashboard_lists_a_registered_machine_after_its_authenticated_heartbeat() {
    let state = AppState::new_in_memory();
    let app = create_router(state.clone());
    let registration = RegisterRequest {
        machine_id: "MACHINE_PRESENCE_01".to_string(),
        hostname: "client-01".to_string(),
        os_version: "Windows 11".to_string(),
        registration_token: state.enrollment_token.clone(),
    };

    let registered = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/machines/register")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&registration).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(registered.status(), StatusCode::OK);
    let registration_body = axum::body::to_bytes(registered.into_body(), usize::MAX)
        .await
        .unwrap();
    let device_jwt = serde_json::from_slice::<serde_json::Value>(&registration_body).unwrap()
        ["device_jwt"]
        .as_str()
        .unwrap()
        .to_string();

    let heartbeat = HeartbeatRequest {
        machine_id: registration.machine_id.clone(),
        disk_usage_pct: 27.5,
        active_session_id: Some("SESSION_LIVE".to_string()),
    };
    let heartbeated = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/machines/heartbeat")
                .header("Authorization", format!("Bearer {device_jwt}"))
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&heartbeat).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(heartbeated.status(), StatusCode::OK);

    // A client that misses the server-side 90s heartbeat window starts a new
    // continuous online interval when it reconnects.
    {
        let mut mem = state.mem_state.write();
        let machine = mem.machines.get_mut(&registration.machine_id).unwrap();
        machine.last_seen_at = Utc::now() - Duration::seconds(91);
        machine.online_since_at = Utc::now() - Duration::hours(2);
    }
    let reconnected = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/machines/heartbeat")
                .header("Authorization", format!("Bearer {device_jwt}"))
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&heartbeat).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reconnected.status(), StatusCode::OK);

    let machine_token_attempt = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/machines")
                .header("Authorization", format!("Bearer {device_jwt}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(machine_token_attempt.status(), StatusCode::UNAUTHORIZED);

    let dashboard_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/machines")
                .header("X-Server-Token", &state.server_api_token)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(dashboard_response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(dashboard_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let machine = &payload["machines"][0];
    assert_eq!(machine["machine_id"], "MACHINE_PRESENCE_01");
    assert_eq!(machine["hostname"], "client-01");
    assert_eq!(machine["status"], "ONLINE");
    assert_eq!(machine["is_online"], true);
    assert_eq!(machine["disk_usage_pct"], 27.5);
    assert_eq!(machine["active_session_id"], "SESSION_LIVE");
    assert!(machine["registered_at"].is_string());
    assert!(machine["last_seen_at"].is_string());
    assert!(machine["online_since_at"].is_string());
    assert!(machine["online_duration_secs"].as_u64().is_some());
    assert!(machine["online_duration_secs"].as_u64().unwrap() < 5);
}

#[tokio::test]
async fn dashboard_session_cookie_allows_machine_reads_without_exposing_the_dashboard_token() {
    let state = AppState::new_in_memory();
    let app = create_router(state.clone());

    let session_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/dashboard/session")
                .header("X-Server-Token", &state.server_api_token)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(session_response.status(), StatusCode::NO_CONTENT);
    let session_cookie = session_response
        .headers()
        .get("Set-Cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();
    assert!(session_response
        .headers()
        .get("Set-Cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .contains("HttpOnly"));
    assert!(session_response
        .headers()
        .get("Set-Cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .contains("Secure"));
    assert!(session_response
        .headers()
        .get("Set-Cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .contains("SameSite=Strict"));

    let dashboard_read = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/machines")
                .header("Cookie", session_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(dashboard_read.status(), StatusCode::OK);
}

#[tokio::test]
async fn dashboard_login_exchanges_a_user_entered_password_for_a_secure_session_cookie() {
    let state = AppState::new_in_memory();
    let app = create_router(state.clone());

    let rejected = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/dashboard/login")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"password":"incorrect"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(rejected.status(), StatusCode::UNAUTHORIZED);

    let accepted = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/dashboard/login")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({ "password": state.server_api_token }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(accepted.status(), StatusCode::NO_CONTENT);
    let set_cookie = accepted.headers().get("Set-Cookie").unwrap().to_str().unwrap();
    assert!(set_cookie.contains("trajectory_dashboard_session="));
    assert!(set_cookie.contains("HttpOnly"));
    assert!(set_cookie.contains("Secure"));
    assert!(set_cookie.contains("SameSite=Strict"));
}

#[tokio::test]
async fn dashboard_assets_are_served_only_under_dashboard_with_spa_fallback() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("index.html"), "<h1>Trajectory Dashboard</h1>").unwrap();
    let mut state = AppState::new_in_memory();
    state.dashboard_assets_dir = temp.path().to_path_buf();
    let app = create_router(state);

    let deep_link = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/dashboard/machines/MACHINE_01")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(deep_link.status(), StatusCode::OK);
    let page = axum::body::to_bytes(deep_link.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(page.as_ref(), b"<h1>Trajectory Dashboard</h1>");

    let api_miss = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/not-found")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(api_miss.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn session_initiation_requires_a_machine_jwt_and_rejects_spoofed_machine_id() {
    let state = AppState::new_in_memory();
    let app = create_router(state.clone());
    let request = InitiateRequest {
        session_id: "SESS_AUTH_REQUIRED".to_string(),
        chunk_count: 1,
        total_size_bytes: 1,
        archive_sha256: hex::encode(Sha256::digest(b"x")),
        machine_id: Some("MACHINE_B".to_string()),
        schema_version: Some("1.0".to_string()),
        user_id: Some("USER_01".to_string()),
    };

    let unauthenticated = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/sessions")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);

    let machine_a_token = create_jwt("MACHINE_A", &state.jwt_secret).unwrap();
    let spoofed = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/sessions")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {machine_a_token}"))
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(spoofed.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn session_chunks_are_only_available_to_the_machine_that_initiated_the_session() {
    let state = AppState::new_in_memory();
    let app = create_router(state.clone());
    let owner_token = create_jwt("MACHINE_OWNER", &state.jwt_secret).unwrap();
    let other_token = create_jwt("MACHINE_OTHER", &state.jwt_secret).unwrap();
    let body = b"owner-only chunk";
    let request = InitiateRequest {
        session_id: "SESS_OWNER_ONLY".to_string(),
        chunk_count: 1,
        total_size_bytes: body.len() as u64,
        archive_sha256: hex::encode(Sha256::digest(body)),
        machine_id: Some("MACHINE_OWNER".to_string()),
        schema_version: Some("1.0".to_string()),
        user_id: Some("USER_01".to_string()),
    };

    let initiated = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/sessions")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {owner_token}"))
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(initiated.status(), StatusCode::OK);

    let forbidden = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/sessions/SESS_OWNER_ONLY/chunks/0")
                .header("Authorization", format!("Bearer {other_token}"))
                .header("X-Chunk-SHA256", hex::encode(Sha256::digest(body)))
                .body(Body::from(body.to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn upload_rejects_a_chunk_without_a_sha256_header() {
    let state = AppState::new_in_memory();
    let app = create_router(state.clone());
    let token = create_jwt("MACHINE_01", &state.jwt_secret).unwrap();
    let payload = b"chunk requiring a checksum";
    let request = InitiateRequest {
        session_id: "SESS_REQUIRED_CHECKSUM".to_string(),
        chunk_count: 1,
        total_size_bytes: payload.len() as u64,
        archive_sha256: hex::encode(Sha256::digest(payload)),
        machine_id: Some("MACHINE_01".to_string()),
        schema_version: Some("1.0".to_string()),
        user_id: Some("USER_01".to_string()),
    };
    let initiated = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/sessions")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(initiated.status(), StatusCode::OK);

    let missing_checksum = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/sessions/SESS_REQUIRED_CHECKSUM/chunks/0")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::from(payload.to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing_checksum.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn completion_rejects_an_archive_with_the_wrong_declared_size() {
    let state = AppState::new_in_memory();
    let app = create_router(state.clone());
    let token = create_jwt("MACHINE_01", &state.jwt_secret).unwrap();
    let payload = b"short archive";
    let request = InitiateRequest {
        session_id: "SESS_SIZE_MISMATCH".to_string(),
        chunk_count: 1,
        total_size_bytes: (payload.len() + 1) as u64,
        archive_sha256: hex::encode(Sha256::digest(payload)),
        machine_id: Some("MACHINE_01".to_string()),
        schema_version: Some("1.0".to_string()),
        user_id: Some("USER_01".to_string()),
    };
    let initiated = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/sessions")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(initiated.status(), StatusCode::OK);

    let uploaded = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/sessions/SESS_SIZE_MISMATCH/chunks/0")
                .header("Authorization", format!("Bearer {token}"))
                .header("X-Chunk-SHA256", hex::encode(Sha256::digest(payload)))
                .body(Body::from(payload.to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(uploaded.status(), StatusCode::OK);

    let completed = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/sessions/SESS_SIZE_MISMATCH/complete")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(completed.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

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
    let token = create_jwt("M01", &state.jwt_secret).unwrap();
    let app = create_router(state);

    let init_req = InitiateRequest {
        session_id: "SESS_ERR_01".to_string(),
        chunk_count: 1,
        total_size_bytes: 100,
        archive_sha256: hex::encode(Sha256::digest(b"chunk real data")),
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
                .header("Authorization", format!("Bearer {token}"))
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
                .header(
                    "X-Chunk-SHA256",
                    "0000000000000000000000000000000000000000000000000000000000000000",
                )
                .header("Authorization", format!("Bearer {token}"))
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
    let token = create_jwt("M01", &state.jwt_secret).unwrap();
    let app = create_router(state);

    let chunk0 = b"chunk_zero_data";
    let chunk0_sha = hex::encode(Sha256::digest(chunk0));

    // Claim archive sha is different
    let init_req = InitiateRequest {
        session_id: "SESS_MISMATCH_01".to_string(),
        chunk_count: 1,
        total_size_bytes: chunk0.len() as u64,
        archive_sha256: "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
            .to_string(),
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
                .header("Authorization", format!("Bearer {token}"))
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
                .header("Authorization", format!("Bearer {token}"))
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
                .header("Authorization", format!("Bearer {token}"))
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
    let token = create_jwt("M01", &state.jwt_secret).unwrap();
    let app = create_router(state);

    let init_req = InitiateRequest {
        session_id: "SESS_MISSING_01".to_string(),
        chunk_count: 3,
        total_size_bytes: 300,
        archive_sha256: "0000000000000000000000000000000000000000000000000000000000000000"
            .to_string(),
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
                .header("Authorization", format!("Bearer {token}"))
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
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
