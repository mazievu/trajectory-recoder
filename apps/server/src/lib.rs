//! Trajectory Ingestion Server library (Axum + PostgreSQL + S3).

use axum::Router;
use axum::extract::{Path, State};
use axum::http::{
    HeaderMap, StatusCode,
    header::{CACHE_CONTROL, SET_COOKIE},
};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{get, post, put};
use bytes::Bytes;
use chrono::{DateTime, Duration, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use subtle::ConstantTimeEq;
use tracing::{error, info, warn};
use tower_http::services::{ServeDir, ServeFile};

#[derive(Debug, Clone)]
pub struct ProductionConfig {
    pub database_url: String,
    pub jwt_secret: String,
    pub enrollment_token: String,
    pub dashboard_api_token: String,
    pub dashboard_assets_dir: PathBuf,
    pub s3_bucket: String,
    pub s3_region: String,
    pub s3_endpoint: String,
    pub s3_access_key: String,
    pub s3_secret_key: String,
}

impl ProductionConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let config = Self {
            database_url: required_env("DATABASE_URL")?,
            jwt_secret: required_env("JWT_SECRET")?,
            enrollment_token: required_env("ENROLLMENT_TOKEN")?,
            dashboard_api_token: required_env("DASHBOARD_API_TOKEN")?,
            dashboard_assets_dir: std::env::var("DASHBOARD_ASSETS_DIR")
                .unwrap_or_else(|_| "/opt/trajectory/dashboard".to_string())
                .into(),
            s3_bucket: required_env("S3_BUCKET")?,
            s3_region: required_env("S3_REGION")?,
            s3_endpoint: required_env("S3_ENDPOINT")?,
            s3_access_key: required_env("S3_ACCESS_KEY")?,
            s3_secret_key: required_env("S3_SECRET_KEY")?,
        };
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.jwt_secret.len() >= 32,
            "JWT_SECRET must contain at least 32 bytes"
        );
        anyhow::ensure!(
            self.enrollment_token.len() >= 16,
            "ENROLLMENT_TOKEN must contain at least 16 bytes"
        );
        anyhow::ensure!(
            self.dashboard_api_token.len() >= 32,
            "DASHBOARD_API_TOKEN must contain at least 32 bytes"
        );
        anyhow::ensure!(
            self.dashboard_api_token != self.jwt_secret
                && self.dashboard_api_token != self.enrollment_token,
            "DASHBOARD_API_TOKEN must be distinct from client authentication secrets"
        );
        anyhow::ensure!(
            self.s3_endpoint.starts_with("https://"),
            "S3_ENDPOINT must use HTTPS in production"
        );
        Ok(())
    }
}

fn required_env(name: &str) -> anyhow::Result<String> {
    let value = std::env::var(name)
        .map_err(|_| anyhow::anyhow!("required environment variable {name} is not set"))?;
    anyhow::ensure!(
        !value.trim().is_empty(),
        "required environment variable {name} is empty"
    );
    Ok(value)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String, // machine_id
    pub exp: usize,
    pub iat: usize,
    pub iss: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct DashboardSessionClaims {
    scope: String,
    exp: usize,
    iat: usize,
    iss: String,
}

#[derive(Debug, Deserialize)]
pub struct DashboardLoginRequest {
    pub password: String,
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

/// A server-side presence record. The registration credential is stored only
/// as a digest; it is never returned by the dashboard API.
#[derive(Debug, Clone)]
pub struct StoredMachine {
    pub registration: RegisterRequest,
    pub registered_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub online_since_at: DateTime<Utc>,
    pub disk_usage_pct: f64,
    pub active_session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MachinePresence {
    pub machine_id: String,
    pub hostname: String,
    pub os_version: String,
    pub registered_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub online_since_at: DateTime<Utc>,
    pub online_duration_secs: u64,
    pub status: String,
    pub is_online: bool,
    pub disk_usage_pct: f64,
    pub active_session_id: Option<String>,
}

const MACHINE_ONLINE_TIMEOUT: Duration = Duration::seconds(90);

impl StoredMachine {
    fn presence_at(&self, now: DateTime<Utc>) -> MachinePresence {
        let is_online = now.signed_duration_since(self.last_seen_at) <= MACHINE_ONLINE_TIMEOUT;
        let online_duration_secs = if is_online {
            now.signed_duration_since(self.online_since_at)
                .num_seconds()
                .max(0) as u64
        } else {
            0
        };
        MachinePresence {
            machine_id: self.registration.machine_id.clone(),
            hostname: self.registration.hostname.clone(),
            os_version: self.registration.os_version.clone(),
            registered_at: self.registered_at,
            last_seen_at: self.last_seen_at,
            online_since_at: self.online_since_at,
            online_duration_secs,
            status: if is_online { "ONLINE" } else { "OFFLINE" }.to_string(),
            is_online,
            disk_usage_pct: self.disk_usage_pct,
            active_session_id: self.active_session_id.clone(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryServerState {
    pub machines: HashMap<String, StoredMachine>,
    pub sessions: HashMap<String, StoredSessionMeta>,
}

#[derive(Clone)]
pub struct AppState {
    pub db: Option<sqlx::PgPool>,
    pub object_store: Arc<dyn object_store::ObjectStore>,
    pub jwt_secret: String,
    pub s3_bucket: String,
    pub enrollment_token: String,
    /// Separate credential for the server dashboard. Device JWTs are never
    /// accepted for cross-machine presence reads.
    pub server_api_token: String,
    pub dashboard_assets_dir: PathBuf,
    pub mem_state: Arc<RwLock<InMemoryServerState>>,
}

impl AppState {
    pub async fn connect_production(config: ProductionConfig) -> anyhow::Result<Self> {
        config.validate()?;
        let db = sqlx::postgres::PgPoolOptions::new()
            .max_connections(20)
            .connect(&config.database_url)
            .await
            .map_err(|error| anyhow::anyhow!("failed to connect to PostgreSQL: {error}"))?;
        sqlx::migrate!("../../server/migrations")
            .run(&db)
            .await
            .map_err(|error| anyhow::anyhow!("failed to apply PostgreSQL migrations: {error}"))?;
        let object_store = object_store::aws::AmazonS3Builder::new()
            .with_bucket_name(&config.s3_bucket)
            .with_region(&config.s3_region)
            .with_endpoint(&config.s3_endpoint)
            .with_access_key_id(&config.s3_access_key)
            .with_secret_access_key(&config.s3_secret_key)
            .build()
            .map_err(|error| anyhow::anyhow!("failed to initialize S3 object store: {error}"))?;

        Ok(Self {
            db: Some(db),
            object_store: Arc::new(object_store),
            jwt_secret: config.jwt_secret,
            s3_bucket: config.s3_bucket,
            enrollment_token: config.enrollment_token,
            server_api_token: config.dashboard_api_token,
            dashboard_assets_dir: config.dashboard_assets_dir,
            mem_state: Arc::new(RwLock::new(InMemoryServerState::default())),
        })
    }

    /// Test fixture only. Production startup always uses `connect_production`.
    pub fn new_in_memory() -> Self {
        Self {
            db: None,
            object_store: Arc::new(object_store::memory::InMemory::new()),
            jwt_secret: "dev_test_jwt_secret_key_123456789".to_string(),
            s3_bucket: "trajectory-archives".to_string(),
            enrollment_token: "test_enrollment_token".to_string(),
            server_api_token: "test_dashboard_api_token_1234567890".to_string(),
            dashboard_assets_dir: PathBuf::from("/opt/trajectory/dashboard"),
            mem_state: Arc::new(RwLock::new(InMemoryServerState::default())),
        }
    }
}

pub fn create_jwt(machine_id: &str, secret: &str) -> Result<String, jsonwebtoken::errors::Error> {
    if secret.trim().is_empty() || machine_id.trim().is_empty() {
        return Err(jsonwebtoken::errors::ErrorKind::InvalidKeyFormat.into());
    }
    let now = Utc::now().timestamp() as usize;
    let exp = now + 3600 * 24 * 365; // 1 year
    let claims = Claims {
        sub: machine_id.to_string(),
        exp,
        iat: now,
        iss: "trajectory-server".to_string(),
    };
    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(secret.as_bytes()),
    )
}

pub fn verify_jwt(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    if secret.trim().is_empty() {
        return Err(jsonwebtoken::errors::ErrorKind::InvalidKeyFormat.into());
    }
    let mut validation = jsonwebtoken::Validation::default();
    validation.set_issuer(&["trajectory-server"]);
    validation.validate_exp = true;
    let token_data = jsonwebtoken::decode::<Claims>(
        token,
        &jsonwebtoken::DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )?;
    Ok(token_data.claims)
}

fn create_dashboard_session(secret: &str) -> Result<String, jsonwebtoken::errors::Error> {
    if secret.trim().is_empty() {
        return Err(jsonwebtoken::errors::ErrorKind::InvalidKeyFormat.into());
    }
    let now = Utc::now().timestamp() as usize;
    let claims = DashboardSessionClaims {
        scope: "dashboard".to_string(),
        // The browser session is intentionally much shorter-lived than a
        // device JWT, which is used by the background client process.
        exp: now + 15 * 60,
        iat: now,
        iss: "trajectory-server".to_string(),
    };
    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(secret.as_bytes()),
    )
}

fn verify_dashboard_session(
    token: &str,
    secret: &str,
) -> Result<(), jsonwebtoken::errors::Error> {
    let mut validation = jsonwebtoken::Validation::default();
    validation.set_issuer(&["trajectory-server"]);
    validation.validate_exp = true;
    let claims = jsonwebtoken::decode::<DashboardSessionClaims>(
        token,
        &jsonwebtoken::DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )?
    .claims;
    if claims.scope == "dashboard" {
        Ok(())
    } else {
        Err(jsonwebtoken::errors::ErrorKind::InvalidToken.into())
    }
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

fn require_machine_id(headers: &HeaderMap, jwt_secret: &str) -> Result<String, StatusCode> {
    extract_machine_id(headers, jwt_secret).ok_or(StatusCode::UNAUTHORIZED)
}

async fn require_session_owner(
    state: &AppState,
    headers: &HeaderMap,
    session_id: &str,
) -> Result<String, StatusCode> {
    let caller_machine = require_machine_id(headers, &state.jwt_secret)?;
    let session_machine = if let Some(pool) = &state.db {
        sqlx::query_scalar::<_, String>("SELECT machine_id FROM sessions WHERE session_id = $1")
            .bind(session_id)
            .fetch_optional(pool)
            .await
            .map_err(|error| {
                error!(%error, session_id, "failed to resolve session owner from PostgreSQL");
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .ok_or(StatusCode::NOT_FOUND)?
    } else {
        state
            .mem_state
            .read()
            .sessions
            .get(session_id)
            .map(|session| session.machine_id.clone())
            .ok_or(StatusCode::NOT_FOUND)?
    };
    if caller_machine != session_machine {
        warn!(
            session_id,
            caller_machine, "machine attempted to access another machine's session"
        );
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(caller_machine)
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
    let dashboard_assets_dir = state.dashboard_assets_dir.clone();
    let dashboard_index = dashboard_assets_dir.join("index.html");
    Router::new()
        .route("/api/v1/health", get(health_handler))
        .route("/api/v1/dashboard/session", post(create_dashboard_session_handler))
        .route("/api/v1/dashboard/login", post(dashboard_login_handler))
        .route("/api/v1/machines", get(list_machines_handler))
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
        // The dashboard is deliberately mounted outside `/api` so its SPA
        // fallback cannot mask an API 404 or error response.
        .nest_service(
            "/dashboard",
            ServeDir::new(dashboard_assets_dir).fallback(ServeFile::new(dashboard_index)),
        )
        .with_state(state)
}

pub async fn health_handler() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::OK,
        Json(serde_json::json!({ "status": "healthy" })),
    )
}

fn require_server_api_token(headers: &HeaderMap, expected_token: &str) -> Result<(), StatusCode> {
    let supplied = headers
        .get("X-Server-Token")
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    if bool::from(supplied.as_bytes().ct_eq(expected_token.as_bytes())) {
        Ok(())
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

fn require_dashboard_access(headers: &HeaderMap, state: &AppState) -> Result<(), StatusCode> {
    if require_server_api_token(headers, &state.server_api_token).is_ok() {
        return Ok(());
    }
    let session_token = headers
        .get("Cookie")
        .and_then(|value| value.to_str().ok())
        .and_then(|cookie_header| {
            cookie_header
                .split(';')
                .map(str::trim)
                .find_map(|cookie| cookie.strip_prefix("trajectory_dashboard_session="))
        })
        .ok_or(StatusCode::UNAUTHORIZED)?;
    verify_dashboard_session(session_token, &state.jwt_secret).map_err(|_| StatusCode::UNAUTHORIZED)
}

/// Exchanges the dashboard bootstrap credential for a short-lived browser
/// session. The credential is not placed in a browser cookie or response body.
pub async fn create_dashboard_session_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    require_server_api_token(&headers, &state.server_api_token)?;
    dashboard_session_response(&state)
}

/// Same-origin browser login. The operator enters the dashboard password; the
/// browser receives only a short-lived session cookie, never that password.
pub async fn dashboard_login_handler(
    State(state): State<AppState>,
    Json(payload): Json<DashboardLoginRequest>,
) -> Result<Response, StatusCode> {
    if !bool::from(
        payload
            .password
            .as_bytes()
            .ct_eq(state.server_api_token.as_bytes()),
    ) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    dashboard_session_response(&state)
}

fn dashboard_session_response(state: &AppState) -> Result<Response, StatusCode> {
    let token = create_dashboard_session(&state.jwt_secret).map_err(|error| {
        error!(%error, "failed to sign dashboard session");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let cookie = format!(
        "trajectory_dashboard_session={token}; Path=/api/v1; Max-Age=900; HttpOnly; Secure; SameSite=Strict"
    );
    Ok((
        StatusCode::NO_CONTENT,
        [(SET_COOKIE, cookie), (CACHE_CONTROL, "no-store".to_string())],
    )
        .into_response())
}

#[derive(sqlx::FromRow)]
struct DatabaseMachinePresence {
    machine_id: String,
    hostname: String,
    os_version: String,
    registered_at: DateTime<Utc>,
    last_seen_at: DateTime<Utc>,
    online_since_at: DateTime<Utc>,
    disk_usage_pct: f32,
    active_session_id: Option<String>,
}

impl DatabaseMachinePresence {
    fn presence_at(self, now: DateTime<Utc>) -> MachinePresence {
        let is_online = now.signed_duration_since(self.last_seen_at) <= MACHINE_ONLINE_TIMEOUT;
        let online_duration_secs = if is_online {
            now.signed_duration_since(self.online_since_at)
                .num_seconds()
                .max(0) as u64
        } else {
            0
        };
        MachinePresence {
            machine_id: self.machine_id,
            hostname: self.hostname,
            os_version: self.os_version,
            registered_at: self.registered_at,
            last_seen_at: self.last_seen_at,
            online_since_at: self.online_since_at,
            online_duration_secs,
            status: if is_online { "ONLINE" } else { "OFFLINE" }.to_string(),
            is_online,
            disk_usage_pct: self.disk_usage_pct.into(),
            active_session_id: self.active_session_id,
        }
    }
}

/// Returns the server dashboard's cross-machine presence view. It deliberately
/// accepts only the dedicated dashboard credential, never a device JWT.
pub async fn list_machines_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    require_dashboard_access(&headers, &state)?;
    let now = Utc::now();

    let machines = if let Some(pool) = &state.db {
        let rows = sqlx::query_as::<_, DatabaseMachinePresence>(
            r#"
            SELECT
                m.machine_id,
                m.hostname,
                m.os_version,
                COALESCE(m.registered_at, CURRENT_TIMESTAMP) AS registered_at,
                COALESCE(m.last_heartbeat_at, m.registered_at, CURRENT_TIMESTAMP) AS last_seen_at,
                COALESCE(m.online_since_at, m.registered_at, CURRENT_TIMESTAMP) AS online_since_at,
                COALESCE(latest.disk_usage_pct, 0.0)::REAL AS disk_usage_pct,
                latest.active_session_id
            FROM machines m
            LEFT JOIN LATERAL (
                SELECT disk_usage_pct, active_session_id
                FROM machine_heartbeats
                WHERE machine_id = m.machine_id
                ORDER BY received_at DESC, heartbeat_id DESC
                LIMIT 1
            ) latest ON TRUE
            ORDER BY m.hostname ASC, m.machine_id ASC
            "#,
        )
        .fetch_all(pool)
        .await
        .map_err(|error| {
            error!(%error, "failed to list machine presence from PostgreSQL");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        rows.into_iter().map(|row| row.presence_at(now)).collect()
    } else {
        let mem = state.mem_state.read();
        let mut machines: Vec<_> = mem
            .machines
            .values()
            .map(|machine| machine.presence_at(now))
            .collect();
        machines.sort_by(|left, right| {
            left.hostname
                .cmp(&right.hostname)
                .then_with(|| left.machine_id.cmp(&right.machine_id))
        });
        machines
    };

    Ok(Json(serde_json::json!({ "machines": machines })))
}

pub async fn register_machine_handler(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if payload.machine_id.trim().is_empty()
        || payload.registration_token.trim().is_empty()
        || !bool::from(
            payload
                .registration_token
                .as_bytes()
                .ct_eq(state.enrollment_token.as_bytes()),
        )
    {
        warn!(machine_id = %payload.machine_id, "machine registration rejected");
        return Err(StatusCode::UNAUTHORIZED);
    }

    info!("Registering machine: {}", payload.machine_id);

    // The enrollment credential authenticates registration but must never be
    // retained as plaintext in PostgreSQL or the process cache.
    let registration_token_digest =
        hex::encode(Sha256::digest(payload.registration_token.as_bytes()));

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
            INSERT INTO machines (
                machine_id, hostname, os_version, registration_token,
                registered_at, last_heartbeat_at, online_since_at, status
            )
            VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, 'ONLINE')
            ON CONFLICT (machine_id) DO UPDATE SET
                hostname = EXCLUDED.hostname,
                os_version = EXCLUDED.os_version,
                registration_token = EXCLUDED.registration_token,
                last_heartbeat_at = CURRENT_TIMESTAMP,
                online_since_at = CURRENT_TIMESTAMP,
                status = 'ONLINE'
            "#,
        )
        .bind(&payload.machine_id)
        .bind(&payload.hostname)
        .bind(&payload.os_version)
        .bind(&registration_token_digest)
        .execute(pool)
        .await;

        if let Err(e) = res {
            error!("Failed to register machine in PostgreSQL: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    let mut stored_payload = payload.clone();
    stored_payload.registration_token = registration_token_digest;
    let mut mem = state.mem_state.write();
    let now = Utc::now();
    let existing_registered_at = mem
        .machines
        .get(&payload.machine_id)
        .map(|machine| machine.registered_at);
    mem.machines.insert(
        payload.machine_id.clone(),
        StoredMachine {
            registration: stored_payload,
            registered_at: existing_registered_at.unwrap_or(now),
            last_seen_at: now,
            online_since_at: now,
            disk_usage_pct: 0.0,
            active_session_id: None,
        },
    );

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
    let caller_machine = require_machine_id(&headers, &state.jwt_secret)?;
    if caller_machine != payload.machine_id {
        warn!(
            "Heartbeat machine_id mismatch: caller {} vs payload {}",
            caller_machine, payload.machine_id
        );
        return Err(StatusCode::FORBIDDEN);
    }

    info!(
        "Heartbeat from machine: {}, disk: {:.1}%",
        payload.machine_id, payload.disk_usage_pct
    );

    let now = Utc::now();
    if let Some(ref pool) = state.db {
        let updated = sqlx::query(
            r#"
            UPDATE machines
            SET
                last_heartbeat_at = CURRENT_TIMESTAMP,
                online_since_at = CASE
                    WHEN last_heartbeat_at < CURRENT_TIMESTAMP - INTERVAL '90 seconds'
                         OR status <> 'ONLINE'
                    THEN CURRENT_TIMESTAMP
                    ELSE COALESCE(online_since_at, CURRENT_TIMESTAMP)
                END,
                status = 'ONLINE'
            WHERE machine_id = $1
            "#,
        )
        .bind(&payload.machine_id)
        .execute(pool)
        .await
        .map_err(|error| {
            error!(%error, "failed to update machine heartbeat timestamp");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        if updated.rows_affected() == 0 {
            return Err(StatusCode::NOT_FOUND);
        }

        sqlx::query(
            r#"
            INSERT INTO machine_heartbeats (machine_id, disk_usage_pct, active_session_id, received_at)
            VALUES ($1, $2, $3, CURRENT_TIMESTAMP)
            "#,
        )
        .bind(&payload.machine_id)
        .bind(payload.disk_usage_pct as f32)
        .bind(&payload.active_session_id)
        .execute(pool)
        .await
        .map_err(|error| {
            error!(%error, "failed to persist heartbeat");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    } else {
        let mut mem = state.mem_state.write();
        let machine = mem
            .machines
            .get_mut(&payload.machine_id)
            .ok_or(StatusCode::NOT_FOUND)?;
        if now.signed_duration_since(machine.last_seen_at) > MACHINE_ONLINE_TIMEOUT {
            machine.online_since_at = now;
        }
        machine.last_seen_at = now;
        machine.disk_usage_pct = payload.disk_usage_pct;
        machine.active_session_id = payload.active_session_id.clone();
    }

    Ok(Json(serde_json::json!({ "status": "ok" })))
}

pub async fn initiate_session_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<InitiateRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let machine_id = require_machine_id(&headers, &state.jwt_secret)?;
    if payload
        .machine_id
        .as_deref()
        .is_some_and(|claimed| claimed != machine_id)
    {
        warn!(session_id = %payload.session_id, caller_machine = %machine_id, "session initiation machine_id mismatch");
        return Err(StatusCode::FORBIDDEN);
    }
    let user_id = payload
        .user_id
        .clone()
        .filter(|id| !id.trim().is_empty())
        .ok_or(StatusCode::BAD_REQUEST)?;
    if payload.session_id.trim().is_empty()
        || payload.chunk_count == 0
        || payload.archive_sha256.len() != 64
        || !payload
            .archive_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(StatusCode::BAD_REQUEST);
    }

    info!(
        "Initiating upload for session {}: {} chunks, {} bytes, machine: {}",
        payload.session_id, payload.chunk_count, payload.total_size_bytes, machine_id
    );

    if let Some(ref pool) = state.db {
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
            WHERE sessions.machine_id = EXCLUDED.machine_id
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

        match res {
            Ok(result) if result.rows_affected() == 0 => return Err(StatusCode::FORBIDDEN),
            Ok(_) => {}
            Err(e) => {
                error!("Failed to record session initiation in PostgreSQL: {}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
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
    require_session_owner(&state, &headers, &session_id).await?;
    let claimed_sha256 = headers
        .get("X-Chunk-SHA256")
        .and_then(|v| v.to_str().ok())
        .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or(StatusCode::BAD_REQUEST)?;

    let computed_digest = hex::encode(Sha256::digest(&body));
    if !claimed_sha256.eq_ignore_ascii_case(&computed_digest) {
        warn!(
            "Chunk {} SHA-256 mismatch for session {}: claimed={}, computed={}",
            chunk_index, session_id, claimed_sha256, computed_digest
        );
        return Err(StatusCode::BAD_REQUEST);
    }

    let (machine_id, expected_chunks) = if let Some(pool) = &state.db {
        sqlx::query_as::<_, (String, i32)>(
            "SELECT machine_id, expected_chunks FROM sessions WHERE session_id = $1",
        )
        .bind(&session_id)
        .fetch_optional(pool)
        .await
        .map_err(|error| {
            error!(%error, session_id, "failed to resolve session metadata from PostgreSQL");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .map(|(machine_id, expected_chunks)| (machine_id, expected_chunks as usize))
        .ok_or(StatusCode::NOT_FOUND)?
    } else {
        let mem = state.mem_state.read();
        mem.sessions
            .get(&session_id)
            .map(|s| (s.machine_id.clone(), s.expected_chunks))
            .ok_or(StatusCode::NOT_FOUND)?
    };
    if chunk_index >= expected_chunks {
        return Err(StatusCode::BAD_REQUEST);
    }

    let storage_key = format_chunk_storage_key(&machine_id, &session_id, chunk_index);
    let object_path = object_store::path::Path::from(storage_key.as_str());

    if let Err(e) = state
        .object_store
        .put(&object_path, body.clone().into())
        .await
    {
        error!(
            "Failed to store chunk {} in object store: {}",
            chunk_index, e
        );
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    if let Some(ref pool) = state.db {
        sqlx::query(
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
        .await
        .map_err(|error| {
            error!(%error, session_id, "failed to persist uploaded chunk metadata");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        sqlx::query(
            r#"
            UPDATE sessions
            SET received_chunks = (SELECT COUNT(*) FROM session_chunks WHERE session_id = $1),
                status = 'UPLOADING'
            WHERE session_id = $1
            "#,
        )
        .bind(&session_id)
        .execute(pool)
        .await
        .map_err(|error| {
            error!(%error, session_id, "failed to update upload progress");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    }

    let mut mem = state.mem_state.write();
    if let Some(sess) = mem.sessions.get_mut(&session_id) {
        sess.received_chunks.insert(chunk_index);
        sess.chunk_storage_keys
            .insert(chunk_index, storage_key.clone());
    } else if state.db.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    info!(
        "Stored chunk {}/{} for session {} at {}",
        chunk_index + 1,
        expected_chunks,
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
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    require_session_owner(&state, &headers, &session_id).await?;
    if let Some(pool) = &state.db {
        let (expected_chunks, status): (i32, String) =
            sqlx::query_as("SELECT expected_chunks, status FROM sessions WHERE session_id = $1")
                .bind(&session_id)
                .fetch_optional(pool)
                .await
                .map_err(|error| {
                    error!(%error, session_id, "failed to read upload status from PostgreSQL");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?
                .ok_or(StatusCode::NOT_FOUND)?;
        let uploaded: Vec<i32> = sqlx::query_scalar(
            "SELECT chunk_index FROM session_chunks WHERE session_id = $1 ORDER BY chunk_index",
        )
        .bind(&session_id)
        .fetch_all(pool)
        .await
        .map_err(|error| {
            error!(%error, session_id, "failed to read uploaded chunks from PostgreSQL");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        let uploaded: Vec<usize> = uploaded.into_iter().map(|index| index as usize).collect();
        let received: HashSet<usize> = uploaded.iter().copied().collect();
        let missing: Vec<usize> = (0..expected_chunks as usize)
            .filter(|index| !received.contains(index))
            .collect();

        return Ok(Json(serde_json::json!({
            "session_id": session_id,
            "uploaded_chunks": uploaded,
            "missing_chunks": missing,
            "is_complete": missing.is_empty(),
            "status": status.to_lowercase(),
        })));
    }
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
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    require_session_owner(&state, &headers, &session_id).await?;
    let (expected_chunks, total_size_bytes, archive_sha256, chunk_keys) = if let Some(pool) =
        &state.db
    {
        let (expected_chunks, total_size_bytes, archive_sha256): (i32, i64, String) = sqlx::query_as(
            "SELECT expected_chunks, total_size_bytes, archive_sha256 FROM sessions WHERE session_id = $1",
        )
        .bind(&session_id)
        .fetch_optional(pool)
        .await
        .map_err(|error| {
            error!(%error, session_id, "failed to read completion metadata from PostgreSQL");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;
        let chunks: Vec<(i32, String)> = sqlx::query_as(
            "SELECT chunk_index, storage_key FROM session_chunks WHERE session_id = $1",
        )
        .bind(&session_id)
        .fetch_all(pool)
        .await
        .map_err(|error| {
            error!(%error, session_id, "failed to read completion chunks from PostgreSQL");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        if chunks.len() < expected_chunks as usize {
            warn!(
                session_id,
                received = chunks.len(),
                expected_chunks,
                "cannot complete session with missing chunks"
            );
            return Err(StatusCode::BAD_REQUEST);
        }
        (
            expected_chunks as usize,
            total_size_bytes as u64,
            archive_sha256,
            chunks
                .into_iter()
                .map(|(index, key)| (index as usize, key))
                .collect(),
        )
    } else {
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
            sess.total_size_bytes,
            sess.archive_sha256.clone(),
            sess.chunk_storage_keys.clone(),
        )
    };

    let mut full_archive_hasher = Sha256::new();
    let mut reconstructed_size_bytes = 0_u64;
    for i in 0..expected_chunks {
        let key = match chunk_keys.get(&i) {
            Some(k) => k.clone(),
            None => {
                warn!(
                    "Missing chunk {} key during completion for session {}",
                    i, session_id
                );
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

        reconstructed_size_bytes =
            reconstructed_size_bytes.saturating_add(chunk_bytes.len() as u64);
        full_archive_hasher.update(&chunk_bytes);
    }

    if reconstructed_size_bytes != total_size_bytes {
        warn!(
            session_id,
            expected = total_size_bytes,
            actual = reconstructed_size_bytes,
            "session archive size mismatch"
        );
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }

    let computed_archive_sha256 = hex::encode(full_archive_hasher.finalize());
    if !archive_sha256.is_empty() && !computed_archive_sha256.eq_ignore_ascii_case(&archive_sha256)
    {
        warn!(
            "Session {} archive SHA-256 mismatch: expected {}, computed {}",
            session_id, archive_sha256, computed_archive_sha256
        );
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }

    if let Some(ref pool) = state.db {
        sqlx::query(
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
        .await
        .map_err(|error| {
            error!(%error, session_id, "failed to accept completed session");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
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
