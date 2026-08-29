//! Trajectory Ingestion Server library (Axum + PostgreSQL + S3).

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Json;
use axum::routing::{get, post, put};
use axum::Router;
use bytes::Bytes;
use chrono::Utc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{error, info, warn};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String, // machine_id
    pub exp: usize,
    pub iat: usize,
    pub iss: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub machine_id: String,
    pub hostname: String,
    pub os_version: String,
    pub registration_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatRequest {
    pub machine_id: String,
    pub disk_usage_pct: f64,
    pub active_session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitiateRequest {
    pub session_id: String,
    #[serde(alias = "total_chunks")]
    pub chunk_count: usize,
    #[serde(alias = "total_bytes")]
    pub total_size_bytes: u64,
    pub archive_sha256: String,
    #[serde(default)]
    pub machine_id: Option<String>,
    #[serde(default)]
    pub schema_version: Option<String>,
    #[serde(default)]
    pub user_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StoredSessionMeta {
    pub session_id: String,
    pub machine_id: String,
    pub user_id: String,
    pub expected_chunks: usize,
    pub total_size_bytes: u64,
    pub archive_sha256: String,
    pub received_chunks: HashSet<usize>,
    pub chunk_storage_keys: HashMap<usize, String>,
    pub is_completed: bool,
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryServerState {
    pub machines: HashMap<String, RegisterRequest>,
    pub sessions: HashMap<String, StoredSessionMeta>,
}

#[derive(Clone)]
pub struct AppState {
    pub db: Option<sqlx::PgPool>,
    pub object_store: Arc<dyn object_store::ObjectStore>,
    pub jwt_secret: String,
    pub s3_bucket: String,
    pub mem_state: Arc<RwLock<InMemoryServerState>>,
}

impl AppState {
    pub fn new_in_memory() -> Self {
        Self {
            db: None,
            object_store: Arc::new(object_store::memory::InMemory::new()),
            jwt_secret: "dev_test_jwt_secret_key_123456789".to_string(),
            s3_bucket: "trajectory-archives".to_string(),
            mem_state: Arc::new(RwLock::new(InMemoryServerState::default())),
        }
    }
}

pub fn create_jwt(machine_id: &str, secret: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let now = Utc::now().timestamp() as usize;
    let exp = now + 3600 * 24 * 365; // 1 year
    let claims = Claims {
        sub: machine_id.to_string(),
        exp,
        iat: now,
        iss: "trajectory-server".to_string(),
    };
    let secret_key = if secret.is_empty() {
        "dev_default_jwt_secret_change_in_production"
    } else {
        secret
    };
    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(secret_key.as_bytes()),
    )
}

pub fn verify_jwt(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let secret_key = if secret.is_empty() {
        "dev_default_jwt_secret_change_in_production"
    } else {
        secret
    };
    let mut validation = jsonwebtoken::Validation::default();
    validation.set_issuer(&["trajectory-server"]);
    validation.validate_exp = true;
    let token_data = jsonwebtoken::decode::<Claims>(
        token,
        &jsonwebtoken::DecodingKey::from_secret(secret_key.as_bytes()),
        &validation,
    )?;
    Ok(token_data.claims)
}

pub fn extract_machine_id(headers: &HeaderMap, jwt_secret: &str) -> Option<String> {
    if let Some(auth_header) = headers.get("Authorization").and_then(|h| h.to_str().ok()) {
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            if let Ok(claims) = verify_jwt(token.trim(), jwt_secret) {
                return Some(claims.sub);
            }
        }
    }
    None
}

pub fn format_chunk_storage_key(machine_id: &str, session_id: &str, chunk_index: usize) -> String {
    let now = Utc::now();
    format!(
        "trajectory/{}/{}/{}/{}/{}/{}/chunk_{:05}.bin",
        machine_id,
        now.format("%Y"),
        now.format("%m"),
        now.format("%d"),
        now.format("%H"),
        session_id,
        chunk_index
    )
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/health", get(health_handler))
        .route("/api/v1/machines/register", post(register_machine_handler))
        .route("/api/v1/machines/heartbeat", post(heartbeat_handler))
        .route("/api/v1/sessions", post(initiate_session_handler))
        .route(
            "/api/v1/sessions/:session_id/chunks/:chunk_index",
            put(upload_chunk_handler),
        )
        .route(
            "/api/v1/sessions/:session_id/upload-status",
            get(session_status_handler),
        )
        .route(
            "/api/v1/sessions/:session_id/complete",
            post(complete_session_handler),
        )
        .with_state(state)
}

pub async fn health_handler() -> (StatusCode, Json<serde_json::Value>) {
    (StatusCode::OK, Json(serde_json::json!({ "status": "healthy" })))
}

pub async fn register_machine_handler(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("Registering machine: {}", payload.machine_id);

    let token = match create_jwt(&payload.machine_id, &state.jwt_secret) {
        Ok(t) => t,
        Err(e) => {
            error!("JWT generation error: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    if let Some(ref pool) = state.db {
        let res = sqlx::query(
            r#"
            INSERT INTO machines (machine_id, hostname, os_version, registration_token, registered_at, last_heartbeat_at, status)
            VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, 'ACTIVE')
            ON CONFLICT (machine_id) DO UPDATE SET
                hostname = EXCLUDED.hostname,
                os_version = EXCLUDED.os_version,
                registration_token = EXCLUDED.registration_token,
                last_heartbeat_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(&payload.machine_id)
        .bind(&payload.hostname)
        .bind(&payload.os_version)
        .bind(&payload.registration_token)
        .execute(pool)
        .await;

        if let Err(e) = res {
            error!("Failed to register machine in PostgreSQL: {}", e);
        }
    }

    let mut mem = state.mem_state.write();
    mem.machines.insert(payload.machine_id.clone(), payload.clone());

    Ok(Json(serde_json::json!({
        "status": "registered",
        "device_jwt": token,
        "token": token,
        "machine_id": payload.machine_id,
    })))
}

pub async fn heartbeat_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<HeartbeatRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let caller_machine = extract_machine_id(&headers, &state.jwt_secret);
    if let Some(ref caller) = caller_machine {
        if caller != &payload.machine_id {
            warn!("Heartbeat machine_id mismatch: caller {} vs payload {}", caller, payload.machine_id);
            return Err(StatusCode::FORBIDDEN);
        }
    } else if !state.mem_state.read().machines.contains_key(&payload.machine_id) && state.db.is_none() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    info!("Heartbeat from machine: {}, disk: {:.1}%", payload.machine_id, payload.disk_usage_pct);

    if let Some(ref pool) = state.db {
        let _ = sqlx::query(
            r#"
            INSERT INTO machine_heartbeats (machine_id, disk_usage_pct, active_session_id, received_at)
            VALUES ($1, $2, $3, CURRENT_TIMESTAMP)
            "#,
        )
        .bind(&payload.machine_id)
        .bind(payload.disk_usage_pct as f32)
        .bind(&payload.active_session_id)
        .execute(pool)
        .await;

        let _ = sqlx::query(
            r#"
            UPDATE machines SET last_heartbeat_at = CURRENT_TIMESTAMP WHERE machine_id = $1
            "#,
        )
        .bind(&payload.machine_id)
        .execute(pool)
        .await;
    }

    Ok(Json(serde_json::json!({ "status": "ok" })))
}

pub async fn initiate_session_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<InitiateRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let machine_id = payload
        .machine_id
        .clone()
        .or_else(|| extract_machine_id(&headers, &state.jwt_secret))
        .unwrap_or_else(|| "default_machine".to_string());
    let user_id = payload.user_id.clone().unwrap_or_else(|| "default_user".to_string());

    info!(
        "Initiating upload for session {}: {} chunks, {} bytes, machine: {}",
        payload.session_id, payload.chunk_count, payload.total_size_bytes, machine_id
    );

    if let Some(ref pool) = state.db {
        let _ = sqlx::query(
            r#"
            INSERT INTO machines (machine_id, hostname, os_version, registration_token, registered_at, last_heartbeat_at, status)
            VALUES ($1, 'unknown', 'unknown', 'auto_registered', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, 'ACTIVE')
            ON CONFLICT (machine_id) DO NOTHING
            "#,
        )
        .bind(&machine_id)
        .execute(pool)
        .await;

        let res = sqlx::query(
            r#"
            INSERT INTO sessions (
                session_id, machine_id, user_id, start_time_utc, status,
                expected_chunks, received_chunks, total_size_bytes, archive_sha256,
                verified_sha256, created_at
            )
            VALUES ($1, $2, $3, CURRENT_TIMESTAMP, 'INITIATED', $4, 0, $5, $6, FALSE, CURRENT_TIMESTAMP)
            ON CONFLICT (session_id) DO UPDATE SET
                expected_chunks = EXCLUDED.expected_chunks,
                total_size_bytes = EXCLUDED.total_size_bytes,
                archive_sha256 = EXCLUDED.archive_sha256
            "#,
        )
        .bind(&payload.session_id)
        .bind(&machine_id)
        .bind(&user_id)
        .bind(payload.chunk_count as i32)
        .bind(payload.total_size_bytes as i64)
        .bind(&payload.archive_sha256)
        .execute(pool)
        .await;

        if let Err(e) = res {
            error!("Failed to record session initiation in PostgreSQL: {}", e);
        }
    }

    let mut mem = state.mem_state.write();
    mem.sessions.insert(
        payload.session_id.clone(),
        StoredSessionMeta {
            session_id: payload.session_id.clone(),
            machine_id,
            user_id,
            expected_chunks: payload.chunk_count,
            total_size_bytes: payload.total_size_bytes,
            archive_sha256: payload.archive_sha256,
            received_chunks: HashSet::new(),
            chunk_storage_keys: HashMap::new(),
            is_completed: false,
            created_at: Utc::now(),
        },
    );

    Ok(Json(serde_json::json!({
        "session_id": payload.session_id,
        "upload_id": uuid::Uuid::new_v4().to_string(),
        "status": "initiated",
        "expected_chunks": payload.chunk_count,
    })))
}

pub async fn upload_chunk_handler(
    State(state): State<AppState>,
    Path((session_id, chunk_index)): Path<(String, usize)>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let claimed_sha256 = headers
        .get("X-Chunk-SHA256")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();

    let computed_digest = hex::encode(Sha256::digest(&body));
    if !claimed_sha256.is_empty() && !claimed_sha256.eq_ignore_ascii_case(&computed_digest) {
        warn!(
            "Chunk {} SHA-256 mismatch for session {}: claimed={}, computed={}",
            chunk_index, session_id, claimed_sha256, computed_digest
        );
        return Err(StatusCode::BAD_REQUEST);
    }

    let machine_id = {
        let mem = state.mem_state.read();
        mem.sessions
            .get(&session_id)
            .map(|s| s.machine_id.clone())
            .unwrap_or_else(|| "default_machine".to_string())
    };

    let storage_key = format_chunk_storage_key(&machine_id, &session_id, chunk_index);
    let object_path = object_store::path::Path::from(storage_key.as_str());

    if let Err(e) = state.object_store.put(&object_path, body.clone().into()).await {
        error!("Failed to store chunk {} in object store: {}", chunk_index, e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    if let Some(ref pool) = state.db {
        let _ = sqlx::query(
            r#"
            INSERT INTO session_chunks (session_id, chunk_index, byte_size, sha256, storage_key, uploaded_at)
            VALUES ($1, $2, $3, $4, $5, CURRENT_TIMESTAMP)
            ON CONFLICT (session_id, chunk_index) DO UPDATE SET
                byte_size = EXCLUDED.byte_size,
                sha256 = EXCLUDED.sha256,
                storage_key = EXCLUDED.storage_key,
                uploaded_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(&session_id)
        .bind(chunk_index as i32)
        .bind(body.len() as i32)
        .bind(&computed_digest)
        .bind(&storage_key)
        .execute(pool)
        .await;

        let _ = sqlx::query(
            r#"
            UPDATE sessions
            SET received_chunks = (SELECT COUNT(*) FROM session_chunks WHERE session_id = $1),
                status = 'UPLOADING'
            WHERE session_id = $1
            "#,
        )
        .bind(&session_id)
        .execute(pool)
        .await;
    }

    let mut mem = state.mem_state.write();
    let sess = mem.sessions.get_mut(&session_id).ok_or(StatusCode::NOT_FOUND)?;
    sess.received_chunks.insert(chunk_index);
    sess.chunk_storage_keys.insert(chunk_index, storage_key.clone());

    info!(
        "Stored chunk {}/{} for session {} at {}",
        chunk_index + 1,
        sess.expected_chunks,
        session_id,
        storage_key
    );

    Ok(Json(serde_json::json!({
        "chunk_index": chunk_index,
        "status": "stored",
        "sha256": computed_digest,
        "storage_key": storage_key,
    })))
}

pub async fn session_status_handler(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mem = state.mem_state.read();
    let sess = mem.sessions.get(&session_id).ok_or(StatusCode::NOT_FOUND)?;

    let mut uploaded: Vec<usize> = sess.received_chunks.iter().copied().collect();
    uploaded.sort();

    let mut missing = Vec::new();
    for i in 0..sess.expected_chunks {
        if !sess.received_chunks.contains(&i) {
            missing.push(i);
        }
    }

    Ok(Json(serde_json::json!({
        "session_id": session_id,
        "uploaded_chunks": uploaded,
        "missing_chunks": missing,
        "is_complete": missing.is_empty(),
        "status": if sess.is_completed { "completed" } else { "uploading" },
    })))
}

pub async fn complete_session_handler(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let (expected_chunks, archive_sha256, chunk_keys) = {
        let mem = state.mem_state.read();
        let sess = mem.sessions.get(&session_id).ok_or(StatusCode::NOT_FOUND)?;

        if sess.received_chunks.len() < sess.expected_chunks {
            warn!(
                "Cannot complete session {}: only received {}/{} chunks",
                session_id,
                sess.received_chunks.len(),
                sess.expected_chunks
            );
            return Err(StatusCode::BAD_REQUEST);
        }

        (
            sess.expected_chunks,
            sess.archive_sha256.clone(),
            sess.chunk_storage_keys.clone(),
        )
    };

    let mut full_archive_hasher = Sha256::new();
    for i in 0..expected_chunks {
        let key = match chunk_keys.get(&i) {
            Some(k) => k.clone(),
            None => {
                warn!("Missing chunk {} key during completion for session {}", i, session_id);
                return Err(StatusCode::BAD_REQUEST);
            }
        };

        let path = object_store::path::Path::from(key.as_str());
        let chunk_bytes = match state.object_store.get(&path).await {
            Ok(res) => match res.bytes().await {
                Ok(b) => b,
                Err(e) => {
                    error!("Failed to fetch chunk bytes from object store: {}", e);
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                }
            },
            Err(e) => {
                error!("Chunk object not found in store: {}: {}", key, e);
                return Err(StatusCode::BAD_REQUEST);
            }
        };

        full_archive_hasher.update(&chunk_bytes);
    }

    let computed_archive_sha256 = hex::encode(full_archive_hasher.finalize());
    if !archive_sha256.is_empty() && !computed_archive_sha256.eq_ignore_ascii_case(&archive_sha256) {
        warn!(
            "Session {} archive SHA-256 mismatch: expected {}, computed {}",
            session_id, archive_sha256, computed_archive_sha256
        );
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }

    if let Some(ref pool) = state.db {
        let _ = sqlx::query(
            r#"
            UPDATE sessions
            SET status = 'ACCEPTED',
                verified_sha256 = TRUE,
                completed_at = CURRENT_TIMESTAMP
            WHERE session_id = $1
            "#,
        )
        .bind(&session_id)
        .execute(pool)
        .await;
    }

    {
        let mut mem = state.mem_state.write();
        if let Some(sess) = mem.sessions.get_mut(&session_id) {
            sess.is_completed = true;
        }
    }

    info!("Session {} successfully verified and accepted!", session_id);

    Ok(Json(serde_json::json!({
        "status": "SESSION_ACCEPTED",
        "session_id": session_id,
        "archive_sha256_verified": true,
    })))
}
