//! Trajectory Resumable Chunk Uploader daemon.
//! Packages finalized sessions into encrypted TAR.Zstd chunks and uploads to Ingestion Server.

use archive::{SessionArchiveManifest, chunk_and_encrypt_archive, create_tar_zstd_archive};
use diagnostics::{DiagnosticsConfig, init_diagnostics};
use spool::{SpoolDirectoryManager, SpoolState};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::{error, info, warn};
use upload_client::{InitiateSessionRequest, UploadClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _guard = init_diagnostics(&DiagnosticsConfig::default());
    info!("Trajectory Uploader starting...");

    let spool_root = std::env::var("SPOOL_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("spool"));
    let spool_mgr = SpoolDirectoryManager::new(&spool_root)?;

    let server_url =
        std::env::var("SERVER_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());
    let mut client = UploadClient::new(&server_url);

    if let Ok(token) = std::env::var("DEVICE_TOKEN") {
        client.set_device_token(token);
    }

    info!(
        "Uploader loop active, target server: {}, monitoring pending_upload and uploading...",
        server_url
    );

    loop {
        // 1. Move any finalizing/ sessions that are ready to pending_upload/
        if let Ok(finalizing_sessions) = spool_mgr.list_sessions(SpoolState::Finalizing) {
            for sid in finalizing_sessions {
                let _ =
                    spool_mgr.transition(&sid, SpoolState::Finalizing, SpoolState::PendingUpload);
            }
        }

        // 2. Process pending uploads (packaging & chunking)
        if let Ok(pending_sessions) = spool_mgr.list_sessions(SpoolState::PendingUpload) {
            for sid in pending_sessions {
                info!("Packaging session for upload: {}", sid);
                let session_dir = spool_mgr.session_path(SpoolState::PendingUpload, &sid);
                let staging_dir = session_dir.join("_packaging");
                let archive_file = staging_dir.join("session.tar.zst");
                let chunks_dir = staging_dir.join("chunks");

                // Compress session directory
                match create_tar_zstd_archive(&session_dir, &archive_file, 3) {
                    Ok((uncompressed, _compressed, file_list)) => {
                        // Chunk & Encrypt (64 MiB = 64 * 1024 * 1024)
                        let chunk_size = 64 * 1024 * 1024;
                        match chunk_and_encrypt_archive(
                            &archive_file,
                            &chunks_dir,
                            &sid,
                            chunk_size,
                            None,
                            uncompressed,
                            file_list,
                        ) {
                            Ok(manifest) => {
                                match spool_mgr.transition(
                                    &sid,
                                    SpoolState::PendingUpload,
                                    SpoolState::Uploading,
                                ) {
                                    Ok(_) => {
                                        info!(
                                            "Session {} chunked into {} chunks and moved to uploading",
                                            sid, manifest.chunk_count
                                        );
                                    }
                                    Err(e) => {
                                        error!(
                                            "Failed to transition session {} to Uploading: {}",
                                            sid, e
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to chunk session {}: {}", sid, e);
                                let _ = spool_mgr.transition(
                                    &sid,
                                    SpoolState::PendingUpload,
                                    SpoolState::Failed,
                                );
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to compress session {}: {}", sid, e);
                        let _ = spool_mgr.transition(
                            &sid,
                            SpoolState::PendingUpload,
                            SpoolState::Failed,
                        );
                    }
                }
            }
        }

        // 3. Process sessions in uploading/ stage
        if let Ok(uploading_sessions) = spool_mgr.list_sessions(SpoolState::Uploading) {
            for sid in uploading_sessions {
                if let Err(e) = process_uploading_session(&spool_mgr, &client, &sid).await {
                    error!("Error during upload processing for session {}: {}", sid, e);
                }
            }
        }

        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn process_uploading_session(
    spool_mgr: &SpoolDirectoryManager,
    client: &UploadClient,
    session_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let session_dir = spool_mgr.session_path(SpoolState::Uploading, session_id);
    let staging_dir = session_dir.join("_packaging");
    let chunks_dir = staging_dir.join("chunks");
    let manifest_path = chunks_dir.join("manifest.json");

    if !manifest_path.exists() {
        warn!(
            "Missing manifest.json for uploading session {}, moving to failed",
            session_id
        );
        let _ = spool_mgr.transition(session_id, SpoolState::Uploading, SpoolState::Failed);
        return Ok(());
    }

    let manifest_content = tokio::fs::read_to_string(&manifest_path).await?;
    let manifest: SessionArchiveManifest = serde_json::from_str(&manifest_content)?;

    // Step A: Initiate session on server
    let initiate_req = InitiateSessionRequest {
        session_id: session_id.to_string(),
        chunk_count: manifest.chunk_count,
        total_size_bytes: manifest.compressed_size_bytes,
        archive_sha256: manifest.archive_sha256.clone(),
        machine_id: None,
        schema_version: Some("1.0".to_string()),
        user_id: None,
    };

    match client.initiate_session(&initiate_req).await {
        Ok(resp) => {
            info!(
                "Initiated upload session {} (status: {})",
                session_id, resp.status
            );
        }
        Err(e) => {
            warn!(
                "Failed to initiate session {} on server: {}. Will retry next cycle.",
                session_id, e
            );
            return Ok(());
        }
    }

    // Step B: Query upload status to check for missing/already uploaded chunks
    let missing_chunks: Vec<usize> = match client.get_upload_status(session_id).await {
        Ok(status) => status.missing_chunks,
        Err(_) => (0..manifest.chunk_count).collect(),
    };

    // Step C: Upload chunks in order
    let mut all_ok = true;
    for chunk_entry in &manifest.chunks {
        if !missing_chunks.contains(&chunk_entry.chunk_index) {
            info!(
                "Chunk {} already uploaded for session {}",
                chunk_entry.chunk_index, session_id
            );
            continue;
        }

        let chunk_path =
            find_chunk_path(&chunks_dir, chunk_entry.chunk_index, &chunk_entry.file_name);
        if !chunk_path.exists() {
            error!(
                "Chunk file not found: {:?} for session {}",
                chunk_path, session_id
            );
            all_ok = false;
            break;
        }

        match client
            .upload_chunk_with_retry(
                session_id,
                chunk_entry.chunk_index,
                &chunk_path,
                &chunk_entry.sha256,
            )
            .await
        {
            Ok(()) => {
                info!(
                    "Chunk {}/{} successfully uploaded for session {}",
                    chunk_entry.chunk_index + 1,
                    manifest.chunk_count,
                    session_id
                );
            }
            Err(e) => {
                error!(
                    "Failed to upload chunk {} for session {}: {}",
                    chunk_entry.chunk_index, session_id, e
                );
                all_ok = false;
                break;
            }
        }
    }

    // Step D: Complete session and verify SESSION_ACCEPTED
    if all_ok {
        match client.complete_session(session_id).await {
            Ok(resp) => {
                if resp.status.eq_ignore_ascii_case("SESSION_ACCEPTED")
                    || resp.status.eq_ignore_ascii_case("accepted")
                {
                    let _ = spool_mgr.transition(
                        session_id,
                        SpoolState::Uploading,
                        SpoolState::Uploaded,
                    );
                    info!(
                        "Session {} verified and accepted by server. Moved to uploaded.",
                        session_id
                    );
                } else {
                    warn!(
                        "Server returned unexpected status '{}' for session {}, moving to failed",
                        resp.status, session_id
                    );
                    let _ =
                        spool_mgr.transition(session_id, SpoolState::Uploading, SpoolState::Failed);
                }
            }
            Err(e) => {
                error!("Failed to complete session {}: {}", session_id, e);
            }
        }
    }

    Ok(())
}

fn find_chunk_path(chunks_dir: &Path, chunk_index: usize, file_name: &str) -> PathBuf {
    let direct = chunks_dir.join(file_name);
    if direct.exists() {
        return direct;
    }
    let p_05 = chunks_dir.join(format!("chunk_{:05}.bin", chunk_index));
    if p_05.exists() {
        return p_05;
    }
    let p_04 = chunks_dir.join(format!("chunk_{:04}.bin", chunk_index));
    if p_04.exists() {
        return p_04;
    }
    direct
}
