use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post, put},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockSessionState {
    pub session_id: String,
    pub total_bytes: usize,
    pub total_chunks: usize,
    pub uploaded_chunks: HashSet<usize>,
    pub chunk_payloads: HashMap<usize, Vec<u8>>,
    pub is_completed: bool,
    pub completed_archive_sha256: Option<String>,
}

#[derive(Clone, Default)]
pub struct MockServerState {
    pub registered_machines: Arc<Mutex<HashMap<String, String>>>, // machine_id -> token
    pub sessions: Arc<Mutex<HashMap<String, MockSessionState>>>,
    pub heartbeat_count: Arc<Mutex<u64>>,
}

pub struct MockServerHandle {
    pub addr: SocketAddr,
    pub shutdown_tx: Option<oneshot::Sender<()>>,
    pub state: MockServerState,
}

impl MockServerHandle {
    pub fn url(&self) -> String {
        format!("http://{}", self.addr)
    }

    pub fn stop(mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

pub async fn start_mock_server() -> Result<MockServerHandle, Box<dyn std::error::Error>> {
    let state = MockServerState::default();

    let app = Router::new()
        .route("/api/v1/machines/register", post(handle_register_machine))
        .route("/api/v1/machines/heartbeat", post(handle_heartbeat))
        .route("/api/v1/sessions", post(handle_create_session))
        .route(
            "/api/v1/sessions/:session_id/chunks/:chunk_index",
            put(handle_upload_chunk),
        )
        .route(
            "/api/v1/sessions/:session_id/upload-status",
            get(handle_upload_status),
        )
        .route(
            "/api/v1/sessions/:session_id/complete",
            post(handle_complete_session),
        )
        .with_state(state.clone());

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await
            .unwrap();
    });

    Ok(MockServerHandle {
        addr,
        shutdown_tx: Some(shutdown_tx),
        state,
    })
}

async fn handle_register_machine(
    State(state): State<MockServerState>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let machine_id = payload
        .get("machine_id")
        .and_then(|v| v.as_str())
        .unwrap_or("machine_default");
    let token = format!("tok_{}", uuid::Uuid::new_v4());
    state
        .registered_machines
        .lock()
        .unwrap()
        .insert(machine_id.to_string(), token.clone());

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "machine_id": machine_id,
            "token": token,
            "status": "registered"
        })),
    )
}

async fn handle_heartbeat(State(state): State<MockServerState>) -> impl IntoResponse {
    let mut count = state.heartbeat_count.lock().unwrap();
    *count += 1;
    (
        StatusCode::OK,
        Json(serde_json::json!({ "status": "ok", "heartbeat_count": *count })),
    )
}

async fn handle_create_session(
    State(state): State<MockServerState>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let session_id = match payload.get("session_id").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Missing session_id" })),
            );
        }
    };
    let total_bytes = payload
        .get("total_bytes")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let total_chunks = payload
        .get("total_chunks")
        .and_then(|v| v.as_u64())
        .unwrap_or(1) as usize;

    let mut sessions = state.sessions.lock().unwrap();
    sessions.insert(
        session_id.clone(),
        MockSessionState {
            session_id: session_id.clone(),
            total_bytes,
            total_chunks,
            uploaded_chunks: HashSet::new(),
            chunk_payloads: HashMap::new(),
            is_completed: false,
            completed_archive_sha256: None,
        },
    );

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "session_id": session_id,
            "status": "created",
            "expected_chunks": total_chunks
        })),
    )
}

async fn handle_upload_chunk(
    State(state): State<MockServerState>,
    Path((session_id, chunk_index)): Path<(String, usize)>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let expected_hash = match headers.get("X-Chunk-SHA256").and_then(|h| h.to_str().ok()) {
        Some(h) => h.to_string(),
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Missing X-Chunk-SHA256 header" })),
            );
        }
    };

    let mut hasher = Sha256::new();
    hasher.update(&body);
    let actual_hash = format!("{:x}", hasher.finalize());

    if actual_hash != expected_hash {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Checksum mismatch",
                "expected": expected_hash,
                "actual": actual_hash
            })),
        );
    }

    let mut sessions = state.sessions.lock().unwrap();
    if let Some(session) = sessions.get_mut(&session_id) {
        session.uploaded_chunks.insert(chunk_index);
        session.chunk_payloads.insert(chunk_index, body.to_vec());
        (
            StatusCode::OK,
            Json(serde_json::json!({
                "session_id": session_id,
                "chunk_index": chunk_index,
                "status": "stored"
            })),
        )
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Session not found" })),
        )
    }
}

async fn handle_upload_status(
    State(state): State<MockServerState>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    let sessions = state.sessions.lock().unwrap();
    if let Some(session) = sessions.get(&session_id) {
        let mut uploaded: Vec<usize> = session.uploaded_chunks.iter().copied().collect();
        uploaded.sort();

        let mut missing: Vec<usize> = Vec::new();
        for i in 0..session.total_chunks {
            if !session.uploaded_chunks.contains(&i) {
                missing.push(i);
            }
        }

        (
            StatusCode::OK,
            Json(serde_json::json!({
                "session_id": session_id,
                "uploaded_chunks": uploaded,
                "missing_chunks": missing,
                "is_complete": missing.is_empty()
            })),
        )
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Session not found" })),
        )
    }
}

async fn handle_complete_session(
    State(state): State<MockServerState>,
    Path(session_id): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let mut sessions = state.sessions.lock().unwrap();
    if let Some(session) = sessions.get_mut(&session_id) {
        let archive_sha256 = payload
            .get("archive_sha256")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        session.is_completed = true;
        session.completed_archive_sha256 = archive_sha256;

        (
            StatusCode::OK,
            Json(serde_json::json!({
                "session_id": session_id,
                "status": "accepted",
                "archive_sha256_verified": true
            })),
        )
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Session not found" })),
        )
    }
}
