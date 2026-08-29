use crate::verifiers::crypto_verifier::CryptoVerifier;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkUploadInfo {
    pub chunk_index: usize,
    pub size_bytes: usize,
    pub sha256: String,
    pub is_uploaded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadSessionPlan {
    pub session_id: String,
    pub total_bytes: usize,
    pub chunk_size: usize,
    pub chunks: Vec<ChunkUploadInfo>,
}

pub struct UploadVerifier;

impl UploadVerifier {
    pub fn plan_chunks(session_id: &str, payload: &[u8], chunk_size: usize) -> UploadSessionPlan {
        let mut chunks = Vec::new();
        let mut offset = 0;
        let mut idx = 0;

        while offset < payload.len() {
            let end = (offset + chunk_size).min(payload.len());
            let slice = &payload[offset..end];
            let hash = CryptoVerifier::compute_sha256_hex(slice);

            chunks.push(ChunkUploadInfo {
                chunk_index: idx,
                size_bytes: slice.len(),
                sha256: hash,
                is_uploaded: false,
            });

            offset = end;
            idx += 1;
        }

        UploadSessionPlan {
            session_id: session_id.to_string(),
            total_bytes: payload.len(),
            chunk_size,
            chunks,
        }
    }

    pub async fn execute_upload(
        server_base_url: &str,
        session_id: &str,
        payload: &[u8],
        chunk_size: usize,
    ) -> Result<UploadSessionPlan, String> {
        let client = Client::new();
        let mut plan = Self::plan_chunks(session_id, payload, chunk_size);

        // 1. Initialize session on server
        let init_url = format!("{}/api/v1/sessions", server_base_url);
        let init_resp = client
            .post(&init_url)
            .json(&serde_json::json!({
                "session_id": session_id,
                "total_bytes": payload.len(),
                "total_chunks": plan.chunks.len(),
            }))
            .send()
            .await
            .map_err(|e| format!("Failed to call init session: {}", e))?;

        if !init_resp.status().is_success() {
            return Err(format!(
                "Init session returned status {}",
                init_resp.status()
            ));
        }

        // 2. Upload each chunk
        let mut offset = 0;
        for chunk in &mut plan.chunks {
            let end = (offset + chunk.size_bytes).min(payload.len());
            let chunk_data = payload[offset..end].to_vec();

            let put_url = format!(
                "{}/api/v1/sessions/{}/chunks/{}",
                server_base_url, session_id, chunk.chunk_index
            );
            let put_resp = client
                .put(&put_url)
                .header("X-Chunk-SHA256", &chunk.sha256)
                .body(chunk_data)
                .send()
                .await
                .map_err(|e| format!("Failed to upload chunk {}: {}", chunk.chunk_index, e))?;

            if !put_resp.status().is_success() {
                return Err(format!(
                    "Upload chunk {} returned status {}",
                    chunk.chunk_index,
                    put_resp.status()
                ));
            }

            chunk.is_uploaded = true;
            offset = end;
        }

        // 3. Complete session
        let complete_url = format!(
            "{}/api/v1/sessions/{}/complete",
            server_base_url, session_id
        );
        let complete_resp = client
            .post(&complete_url)
            .json(&serde_json::json!({
                "session_id": session_id,
                "archive_sha256": CryptoVerifier::compute_sha256_hex(payload),
            }))
            .send()
            .await
            .map_err(|e| format!("Failed to call complete session: {}", e))?;

        if !complete_resp.status().is_success() {
            return Err(format!(
                "Complete session returned status {}",
                complete_resp.status()
            ));
        }

        Ok(plan)
    }
}
