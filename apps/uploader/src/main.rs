//! Trajectory Resumable Chunk Uploader daemon.
//! Packages finalized sessions into encrypted TAR.Zstd chunks and uploads to Ingestion Server.

use archive::{SessionArchiveManifest, chunk_and_encrypt_archive, create_tar_zstd_archive};
use diagnostics::{DiagnosticsConfig, init_diagnostics};
use spool::{SpoolDirectoryManager, SpoolState};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tracing::{error, info, warn};
use upload_client::{
    HeartbeatRequest, InitiateSessionRequest, RegisterMachineRequest, UploadClient,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeRole {
    Client,
    Server,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ClientRuntimeConfig {
    server_url: String,
    machine_id: String,
}

impl ClientRuntimeConfig {
    fn from_env() -> Result<Self, String> {
        Self::from_values(
            std::env::var("TRAJECTORY_SERVER_URL").ok(),
            std::env::var("TRAJECTORY_MACHINE_ID").ok(),
        )
    }

    fn from_values(server_url: Option<String>, machine_id: Option<String>) -> Result<Self, String> {
        let server_url = server_url
            .map(|value| value.trim().trim_end_matches('/').to_string())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                "TRAJECTORY_SERVER_URL is required; refusing to guess a collector endpoint"
                    .to_string()
            })?;
        validate_server_url(&server_url)?;

        let machine_id = machine_id
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                "TRAJECTORY_MACHINE_ID is required; refusing an un-enrolled client".to_string()
            })?;

        Ok(Self {
            server_url,
            machine_id,
        })
    }
}

fn validate_server_url(server_url: &str) -> Result<(), String> {
    let (scheme, remainder) = server_url
        .split_once("://")
        .ok_or_else(|| "TRAJECTORY_SERVER_URL must include http:// or https://".to_string())?;
    let authority = remainder.split('/').next().unwrap_or_default();
    if authority.is_empty() || authority.chars().any(char::is_whitespace) {
        return Err("TRAJECTORY_SERVER_URL must contain a valid host".to_string());
    }

    let host = authority
        .trim_start_matches('[')
        .split(']')
        .next()
        .unwrap_or(authority)
        .split(':')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let is_loopback = matches!(host.as_str(), "localhost" | "127.0.0.1" | "::1");

    match scheme {
        "https" => Ok(()),
        "http" if is_loopback => Ok(()),
        "http" => Err(
            "TRAJECTORY_SERVER_URL must use HTTPS outside a loopback test environment".to_string(),
        ),
        _ => Err("TRAJECTORY_SERVER_URL must use http:// or https://".to_string()),
    }
}

fn resolve_runtime_role(
    role_override: Option<&str>,
    executable: &Path,
) -> Result<RuntimeRole, String> {
    let executable_name = executable
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let executable_role = if executable_name.contains("server") {
        RuntimeRole::Server
    } else if executable_name.contains("uploader")
        || executable_name.contains("agent")
        || executable_name.contains("supervisor")
    {
        RuntimeRole::Client
    } else {
        return Err(
            "cannot infer deployment role from executable name; set DEPLOYMENT_ROLE".to_string(),
        );
    };

    let requested_role = match role_override.map(str::trim).filter(|role| !role.is_empty()) {
        None => executable_role,
        Some("client") => RuntimeRole::Client,
        Some("server") => RuntimeRole::Server,
        Some(_) => return Err("DEPLOYMENT_ROLE must be either client or server".to_string()),
    };

    if requested_role != executable_role {
        return Err(
            "DEPLOYMENT_ROLE conflicts with this executable; refusing to run the wrong role"
                .to_string(),
        );
    }
    Ok(executable_role)
}

fn machine_hostname(machine_id: &str) -> String {
    std::env::var("COMPUTERNAME")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| machine_id.to_string())
}

fn token_path(spool_root: &Path) -> PathBuf {
    std::env::var("TRAJECTORY_DEVICE_TOKEN_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| spool_root.join("device-token.dpapi"))
}

fn load_device_token(path: &Path) -> Result<Option<String>, String> {
    match fs::read(path) {
        Ok(ciphertext) => unprotect_token_for_current_user(&ciphertext).map(Some),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!(
            "could not read protected device credential: {error}"
        )),
    }
}

fn store_device_token(path: &Path, token: &str) -> Result<(), String> {
    let encrypted = protect_token_for_current_user(token.as_bytes())?;
    let parent = path
        .parent()
        .ok_or_else(|| "device token path has no parent directory".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("could not create credential directory: {error}"))?;
    fs::write(path, encrypted)
        .map_err(|error| format!("could not store protected device credential: {error}"))
}

#[cfg(windows)]
fn protect_token_for_current_user(plaintext: &[u8]) -> Result<Vec<u8>, String> {
    use windows::Win32::Foundation::{HLOCAL, LocalFree};
    use windows::Win32::Security::Cryptography::{
        CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData,
    };
    use windows::core::PCWSTR;

    let size =
        u32::try_from(plaintext.len()).map_err(|_| "device credential is too large".to_string())?;
    let input = CRYPT_INTEGER_BLOB {
        cbData: size,
        pbData: plaintext.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    unsafe {
        CryptProtectData(
            &input,
            PCWSTR::null(),
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
        .map_err(|error| format!("DPAPI encryption failed: {error}"))?;
        let bytes = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        let _ = LocalFree(HLOCAL(output.pbData.cast()));
        Ok(bytes)
    }
}

#[cfg(windows)]
fn unprotect_token_for_current_user(ciphertext: &[u8]) -> Result<String, String> {
    use windows::Win32::Foundation::{HLOCAL, LocalFree};
    use windows::Win32::Security::Cryptography::{
        CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptUnprotectData,
    };

    let size = u32::try_from(ciphertext.len())
        .map_err(|_| "protected device credential is too large".to_string())?;
    let input = CRYPT_INTEGER_BLOB {
        cbData: size,
        pbData: ciphertext.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    unsafe {
        CryptUnprotectData(
            &input,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
        .map_err(|error| format!("DPAPI decryption failed: {error}"))?;
        let bytes = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        let _ = LocalFree(HLOCAL(output.pbData.cast()));
        String::from_utf8(bytes)
            .map_err(|_| "protected device credential is not valid UTF-8".to_string())
    }
}

#[cfg(not(windows))]
fn protect_token_for_current_user(_plaintext: &[u8]) -> Result<Vec<u8>, String> {
    Err("protected device credentials are only supported on Windows clients".to_string())
}

#[cfg(not(windows))]
fn unprotect_token_for_current_user(_ciphertext: &[u8]) -> Result<String, String> {
    Err("protected device credentials are only supported on Windows clients".to_string())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _guard = init_diagnostics(&DiagnosticsConfig::default());
    let executable = std::env::current_exe()?;
    let role = resolve_runtime_role(
        std::env::var("DEPLOYMENT_ROLE").ok().as_deref(),
        &executable,
    )?;
    if role != RuntimeRole::Client {
        return Err("trajectory-uploader is a client-only background process".into());
    }

    let spool_root = std::env::var("SPOOL_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("spool"));
    let spool_mgr = SpoolDirectoryManager::new(&spool_root)?;
    let runtime = ClientRuntimeConfig::from_env()?;
    let mut client = UploadClient::new(&runtime.server_url);
    let credential_path = token_path(&spool_root);

    if let Some(token) = std::env::var("DEVICE_TOKEN")
        .ok()
        .filter(|token| !token.trim().is_empty())
    {
        client.set_device_token(token);
    } else if let Some(token) = load_device_token(&credential_path)? {
        client.set_device_token(token);
    } else {
        let enrollment_token = std::env::var("TRAJECTORY_ENROLLMENT_TOKEN").map_err(|_| {
            "a client needs DEVICE_TOKEN, a DPAPI credential, or TRAJECTORY_ENROLLMENT_TOKEN"
                .to_string()
        })?;
        let registration = client
            .register_machine(&RegisterMachineRequest {
                machine_id: runtime.machine_id.clone(),
                hostname: machine_hostname(&runtime.machine_id),
                os_version: std::env::consts::OS.to_string(),
                registration_token: enrollment_token,
            })
            .await?;
        if registration.device_jwt.trim().is_empty() {
            return Err("server returned an empty device credential".into());
        }
        store_device_token(&credential_path, &registration.device_jwt)?;
        client.set_device_token(registration.device_jwt);
    }

    info!(
        machine_id = %runtime.machine_id,
        server = %runtime.server_url,
        "Client uploader is running in the background"
    );

    let mut last_heartbeat = Instant::now() - Duration::from_secs(30);

    loop {
        if last_heartbeat.elapsed() >= Duration::from_secs(30) {
            let active_session_id = spool_mgr
                .list_sessions(SpoolState::Recording)
                .ok()
                .and_then(|sessions| sessions.into_iter().next());
            let heartbeat = HeartbeatRequest {
                machine_id: runtime.machine_id.clone(),
                disk_usage_pct: disk_usage_percent(&spool_root),
                active_session_id,
            };
            if let Err(error) = client.send_heartbeat(&heartbeat).await {
                warn!(machine_id = %runtime.machine_id, error = %error, "client heartbeat failed; will retry");
            }
            last_heartbeat = Instant::now();
        }

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
                if let Err(e) =
                    process_uploading_session(&spool_mgr, &client, &sid, &runtime.machine_id).await
                {
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
    enrolled_machine_id: &str,
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
    let session_identity = read_session_identity(&session_dir)?;
    if session_identity.machine_id != enrolled_machine_id {
        return Err(format!(
            "refusing upload: session {} belongs to {}, not enrolled client {}",
            session_id, session_identity.machine_id, enrolled_machine_id
        )
        .into());
    }

    // Step A: Initiate session on server
    let initiate_req = InitiateSessionRequest {
        session_id: session_id.to_string(),
        chunk_count: manifest.chunk_count,
        total_size_bytes: manifest.compressed_size_bytes,
        archive_sha256: manifest.archive_sha256.clone(),
        machine_id: Some(session_identity.machine_id),
        schema_version: Some("1.0".to_string()),
        user_id: Some(session_identity.user_id),
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

#[derive(Debug, serde::Deserialize)]
struct SessionIdentity {
    machine_id: String,
    user_id: String,
}

fn read_session_identity(
    session_dir: &Path,
) -> Result<SessionIdentity, Box<dyn std::error::Error>> {
    let manifest_path = session_dir.join("manifest.json");
    let contents = fs::read_to_string(&manifest_path)?;
    let identity: SessionIdentity = serde_json::from_str(&contents)?;
    if identity.machine_id.trim().is_empty() || identity.user_id.trim().is_empty() {
        return Err("session manifest is missing machine_id or user_id".into());
    }
    Ok(identity)
}

#[cfg(windows)]
fn disk_usage_percent(path: &Path) -> f64 {
    use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    use windows::core::HSTRING;

    let path = HSTRING::from(path.to_string_lossy().as_ref());
    let mut available = 0u64;
    let mut total = 0u64;
    let mut total_free = 0u64;
    unsafe {
        if GetDiskFreeSpaceExW(
            &path,
            Some(&mut available),
            Some(&mut total),
            Some(&mut total_free),
        )
        .is_ok()
            && total > 0
        {
            return ((total - total_free) as f64 / total as f64) * 100.0;
        }
    }
    0.0
}

#[cfg(not(windows))]
fn disk_usage_percent(_path: &Path) -> f64 {
    0.0
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn client_runtime_requires_an_explicit_server_endpoint() {
        let err = ClientRuntimeConfig::from_values(None, Some("MACHINE-01".to_string()))
            .expect_err("a client must never guess a server endpoint");
        assert!(err.contains("TRAJECTORY_SERVER_URL"));
    }

    #[test]
    fn client_runtime_rejects_plain_http_to_non_loopback_hosts() {
        let err = ClientRuntimeConfig::from_values(
            Some("http://collector.internal:8080".to_string()),
            Some("MACHINE-01".to_string()),
        )
        .expect_err("remote telemetry must not use plaintext HTTP");
        assert!(err.contains("HTTPS"));
    }

    #[test]
    fn client_runtime_accepts_https_and_strips_trailing_slash() {
        let config = ClientRuntimeConfig::from_values(
            Some("https://collector.example.test/".to_string()),
            Some("MACHINE-01".to_string()),
        )
        .expect("explicit HTTPS collector is a valid client target");
        assert_eq!(config.server_url, "https://collector.example.test");
        assert_eq!(config.machine_id, "MACHINE-01");
    }

    #[test]
    fn executable_name_defaults_to_its_safe_runtime_role() {
        assert_eq!(
            resolve_runtime_role(None, Path::new("trajectory-uploader.exe")).unwrap(),
            RuntimeRole::Client
        );
        assert_eq!(
            resolve_runtime_role(None, Path::new("trajectory-server.exe")).unwrap(),
            RuntimeRole::Server
        );
    }

    #[test]
    fn client_executable_refuses_a_server_role_override() {
        assert!(
            resolve_runtime_role(Some("server"), Path::new("trajectory-uploader.exe")).is_err()
        );
        assert!(
            resolve_runtime_role(Some("unknown"), Path::new("trajectory-uploader.exe")).is_err()
        );
    }

    #[test]
    fn session_identity_is_read_from_the_recorded_session_manifest() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("manifest.json"),
            r#"{"machine_id":"MACHINE-01","user_id":"operator-01"}"#,
        )
        .unwrap();

        let identity = read_session_identity(dir.path()).unwrap();
        assert_eq!(identity.machine_id, "MACHINE-01");
        assert_eq!(identity.user_id, "operator-01");
    }

    #[cfg(windows)]
    #[test]
    fn persisted_device_credential_is_bound_to_windows_dpapi() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("device-token.dpapi");
        store_device_token(&path, "device-jwt-secret").unwrap();

        assert_ne!(fs::read(&path).unwrap(), b"device-jwt-secret");
        assert_eq!(
            load_device_token(&path).unwrap().as_deref(),
            Some("device-jwt-secret")
        );
    }
}
