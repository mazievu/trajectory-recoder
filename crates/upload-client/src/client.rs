use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::time::Duration;
use tracing::warn;

#[derive(Debug, Clone)]
pub struct UploadClientConfig {
    pub max_retries: usize,
    pub initial_retry_backoff_ms: u64,
    pub max_retry_backoff_ms: u64,
    pub request_timeout: Duration,
    pub connect_timeout: Duration,
}

impl Default for UploadClientConfig {
    fn default() -> Self {
        Self {
            max_retries: 10,
            initial_retry_backoff_ms: 1000,
            max_retry_backoff_ms: 60000,
            request_timeout: Duration::from_secs(60),
            connect_timeout: Duration::from_secs(10),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum UploadError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Server returned error (HTTP {status}): {message}")]
    Server { status: u16, message: String },

    #[error("Max retries exceeded ({0} attempts)")]
    MaxRetriesExceeded(usize),

    #[error("Chunk checksum mismatch: expected {expected}, computed {computed}")]
    ChecksumMismatch { expected: String, computed: String },

    #[error("Unexpected session status from server: {0}")]
    UnexpectedStatus(String),

    #[error("Other error: {0}")]
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterMachineRequest {
    pub machine_id: String,
    pub hostname: String,
    pub os_version: String,
    pub registration_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterMachineResponse {
    #[serde(default)]
    pub status: String,
    #[serde(alias = "token", alias = "device_token")]
    pub device_jwt: String,
    #[serde(default)]
    pub machine_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatRequest {
    pub machine_id: String,
    pub disk_usage_pct: f64,
    pub active_session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitiateSessionRequest {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitiateSessionResponse {
    pub session_id: String,
    #[serde(default)]
    pub upload_id: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub expected_chunks: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStatusResponse {
    pub session_id: String,
    pub uploaded_chunks: Vec<usize>,
    pub missing_chunks: Vec<usize>,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub is_complete: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteSessionResponse {
    pub status: String,
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub archive_sha256_verified: bool,
}

#[derive(Clone)]
pub struct UploadClient {
    client: Client,
    server_base_url: String,
    device_token: Option<String>,
    config: UploadClientConfig,
}

impl UploadClient {
    pub fn new(server_base_url: impl Into<String>) -> Self {
        Self::with_config(server_base_url, UploadClientConfig::default())
    }

    pub fn with_config(server_base_url: impl Into<String>, config: UploadClientConfig) -> Self {
        let client = Client::builder()
            .timeout(config.request_timeout)
            .connect_timeout(config.connect_timeout)
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            server_base_url: server_base_url.into().trim_end_matches('/').to_string(),
            device_token: None,
            config,
        }
    }

    pub fn with_device_token(mut self, token: impl Into<String>) -> Self {
        self.device_token = Some(token.into());
        self
    }

    pub fn set_device_token(&mut self, token: impl Into<String>) {
        self.device_token = Some(token.into());
    }

    pub fn device_token(&self) -> Option<&str> {
        self.device_token.as_deref()
    }

    pub fn server_url(&self) -> &str {
        &self.server_base_url
    }

    pub async fn register_machine(
        &self,
        req: &RegisterMachineRequest,
    ) -> Result<RegisterMachineResponse, UploadError> {
        let url = format!("{}/api/v1/machines/register", self.server_base_url);
        let resp = self.client.post(&url).json(req).send().await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(UploadError::Server {
                status,
                message: body,
            });
        }

        let res: RegisterMachineResponse = resp.json().await?;
        Ok(res)
    }

    pub async fn send_heartbeat(&self, req: &HeartbeatRequest) -> Result<(), UploadError> {
        self.send_heartbeat_with_token(req, self.device_token.as_deref().unwrap_or_default())
            .await
    }

    pub async fn send_heartbeat_with_token(
        &self,
        req: &HeartbeatRequest,
        token: &str,
    ) -> Result<(), UploadError> {
        let url = format!("{}/api/v1/machines/heartbeat", self.server_base_url);
        let mut request = self.client.post(&url);
        if !token.is_empty() {
            request = request.bearer_auth(token);
        }
        let resp = request.json(req).send().await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(UploadError::Server {
                status,
                message: body,
            });
        }

        Ok(())
    }

    pub async fn initiate_session(
        &self,
        req: &InitiateSessionRequest,
    ) -> Result<InitiateSessionResponse, UploadError> {
        self.initiate_session_with_token(req, self.device_token.as_deref().unwrap_or_default())
            .await
    }

    pub async fn initiate_session_with_token(
        &self,
        req: &InitiateSessionRequest,
        token: &str,
    ) -> Result<InitiateSessionResponse, UploadError> {
        let url = format!("{}/api/v1/sessions", self.server_base_url);
        let mut request = self.client.post(&url);
        if !token.is_empty() {
            request = request.bearer_auth(token);
        }
        let resp = request.json(req).send().await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(UploadError::Server {
                status,
                message: body,
            });
        }

        let res: InitiateSessionResponse = resp.json().await?;
        Ok(res)
    }

    pub async fn initiate_upload(
        &self,
        req: &InitiateSessionRequest,
        token: &str,
    ) -> Result<InitiateSessionResponse, UploadError> {
        self.initiate_session_with_token(req, token).await
    }

    pub async fn upload_chunk_with_retry(
        &self,
        session_id: &str,
        chunk_index: usize,
        chunk_path: impl AsRef<Path>,
        expected_sha256: &str,
    ) -> Result<(), UploadError> {
        let chunk_data = tokio::fs::read(chunk_path.as_ref()).await?;
        self.upload_chunk_bytes_with_retry(session_id, chunk_index, chunk_data, expected_sha256)
            .await
    }

    pub async fn upload_chunk_bytes_with_retry(
        &self,
        session_id: &str,
        chunk_index: usize,
        chunk_data: Vec<u8>,
        expected_sha256: &str,
    ) -> Result<(), UploadError> {
        let computed_sha256 = hex::encode(Sha256::digest(&chunk_data));
        if !expected_sha256.is_empty() && !expected_sha256.eq_ignore_ascii_case(&computed_sha256) {
            return Err(UploadError::ChecksumMismatch {
                expected: expected_sha256.to_string(),
                computed: computed_sha256,
            });
        }

        let url = format!(
            "{}/api/v1/sessions/{}/chunks/{}",
            self.server_base_url, session_id, chunk_index
        );

        let mut backoff_ms = self.config.initial_retry_backoff_ms;
        let max_retries = self.config.max_retries;

        for attempt in 0..max_retries {
            let mut req = self
                .client
                .put(&url)
                .header("X-Chunk-SHA256", &computed_sha256)
                .header("X-Chunk-Size", chunk_data.len().to_string())
                .header("Content-Type", "application/octet-stream")
                .body(chunk_data.clone());

            if let Some(ref token) = self.device_token {
                if !token.is_empty() {
                    req = req.bearer_auth(token);
                }
            }

            let res = req.send().await;

            match res {
                Ok(resp) if resp.status().is_success() => {
                    return Ok(());
                }
                Ok(resp) => {
                    warn!(
                        "Chunk upload attempt {} for session {} chunk {} failed: HTTP {}",
                        attempt + 1,
                        session_id,
                        chunk_index,
                        resp.status()
                    );
                }
                Err(e) => {
                    warn!(
                        "Chunk upload attempt {} for session {} chunk {} error: {}",
                        attempt + 1,
                        session_id,
                        chunk_index,
                        e
                    );
                }
            }

            if attempt + 1 < max_retries {
                // Exponential backoff with random jitter: (rand::random::<f64>() * 0.25 * backoff_ms as f64) as u64
                let jitter = (rand::random::<f64>() * 0.25 * backoff_ms as f64) as u64;
                tokio::time::sleep(Duration::from_millis(backoff_ms + jitter)).await;
                backoff_ms = (backoff_ms * 2).min(self.config.max_retry_backoff_ms).min(60_000);
            }
        }

        Err(UploadError::MaxRetriesExceeded(max_retries))
    }

    pub async fn get_upload_status(
        &self,
        session_id: &str,
    ) -> Result<SessionStatusResponse, UploadError> {
        self.get_upload_status_with_token(
            session_id,
            self.device_token.as_deref().unwrap_or_default(),
        )
        .await
    }

    pub async fn get_upload_status_with_token(
        &self,
        session_id: &str,
        token: &str,
    ) -> Result<SessionStatusResponse, UploadError> {
        let url = format!(
            "{}/api/v1/sessions/{}/upload-status",
            self.server_base_url, session_id
        );
        let mut req = self.client.get(&url);
        if !token.is_empty() {
            req = req.bearer_auth(token);
        }
        let resp = req.send().await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(UploadError::Server {
                status,
                message: body,
            });
        }

        let res: SessionStatusResponse = resp.json().await?;
        Ok(res)
    }

    pub async fn check_status(
        &self,
        session_id: &str,
        token: &str,
    ) -> Result<SessionStatusResponse, UploadError> {
        self.get_upload_status_with_token(session_id, token).await
    }

    pub async fn complete_session(
        &self,
        session_id: &str,
    ) -> Result<CompleteSessionResponse, UploadError> {
        self.complete_session_with_token(
            session_id,
            self.device_token.as_deref().unwrap_or_default(),
        )
        .await
    }

    pub async fn complete_session_with_token(
        &self,
        session_id: &str,
        token: &str,
    ) -> Result<CompleteSessionResponse, UploadError> {
        let url = format!("{}/api/v1/sessions/{}/complete", self.server_base_url, session_id);
        let mut req = self.client.post(&url);
        if !token.is_empty() {
            req = req.bearer_auth(token);
        }
        let resp = req.send().await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(UploadError::Server {
                status,
                message: body,
            });
        }

        let res: CompleteSessionResponse = resp.json().await?;
        if !res.status.eq_ignore_ascii_case("SESSION_ACCEPTED")
            && !res.status.eq_ignore_ascii_case("accepted")
        {
            return Err(UploadError::UnexpectedStatus(res.status));
        }

        Ok(res)
    }

    pub async fn complete_upload(
        &self,
        session_id: &str,
        token: &str,
    ) -> Result<CompleteSessionResponse, UploadError> {
        self.complete_session_with_token(session_id, token).await
    }
}
