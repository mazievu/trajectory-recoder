use crate::manifest::{ArchiveChunkEntry, SessionArchiveManifest};
use chrono::Utc;
use crypto::aead::XChaCha20Aead;
use crypto::hash::compute_sha256;
use crypto::MasterKey;
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Splits a file into fixed-size chunks (e.g. 64 MiB), optionally encrypts each chunk with XChaCha20-Poly1305, and produces manifests.
pub fn chunk_and_encrypt_archive(
    archive_path: impl AsRef<Path>,
    output_chunks_dir: impl AsRef<Path>,
    session_id: &str,
    chunk_size_bytes: usize,
    encryption_key: Option<&[u8; 32]>,
    uncompressed_size: u64,
    file_list: Vec<String>,
) -> std::io::Result<SessionArchiveManifest> {
    let src = archive_path.as_ref();
    let out_dir = output_chunks_dir.as_ref();
    std::fs::create_dir_all(out_dir)?;

    let mut file = File::open(src)?;
    let mut total_archive_bytes = Vec::new();
    file.read_to_end(&mut total_archive_bytes)?;

    let archive_sha256 = hex::encode(compute_sha256(&total_archive_bytes));
    let compressed_size = total_archive_bytes.len() as u64;

    let mut chunk_entries = Vec::new();
    let mut chunk_idx = 0;

    for chunk_slice in total_archive_bytes.chunks(chunk_size_bytes) {
        let chunk_data = if let Some(key) = encryption_key {
            let aad = format!("{}_chunk_{}", session_id, chunk_idx);
            let master_key = MasterKey::from_bytes(*key);
            XChaCha20Aead::encrypt(&master_key, chunk_slice, aad.as_bytes())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?
        } else {
            chunk_slice.to_vec()
        };

        let chunk_sha256 = hex::encode(compute_sha256(&chunk_data));
        let chunk_filename = format!("chunk_{:04}.bin", chunk_idx);
        let chunk_path = out_dir.join(&chunk_filename);

        std::fs::write(&chunk_path, &chunk_data)?;

        chunk_entries.push(ArchiveChunkEntry {
            chunk_index: chunk_idx,
            file_name: chunk_filename,
            byte_size: chunk_data.len(),
            sha256: chunk_sha256,
        });

        chunk_idx += 1;
    }

    let manifest = SessionArchiveManifest {
        session_id: session_id.to_string(),
        created_at_utc: Utc::now().to_rfc3339(),
        uncompressed_size_bytes: uncompressed_size,
        compressed_size_bytes: compressed_size,
        is_encrypted: encryption_key.is_some(),
        archive_sha256,
        chunk_count: chunk_entries.len(),
        chunk_size_bytes,
        chunks: chunk_entries,
        file_list,
    };

    let manifest_path = out_dir.join("manifest.json");
    let manifest_json = serde_json::to_string_pretty(&manifest)?;
    std::fs::write(manifest_path, manifest_json)?;

    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_chunking_and_manifest_generation() {
        let dir = tempdir().unwrap();
        let archive_file = dir.path().join("test.tar.zst");
        let chunks_dir = dir.path().join("chunks");

        // 256KB sample archive
        let data = vec![0xABu8; 256 * 1024];
        std::fs::write(&archive_file, &data).unwrap();

        let key = [7u8; 32];
        let chunk_size = 64 * 1024; // 64KB chunks -> 4 chunks

        let manifest = chunk_and_encrypt_archive(
            &archive_file,
            &chunks_dir,
            "SESS_TEST",
            chunk_size,
            Some(&key),
            500 * 1024,
            vec!["events.raw.ndjson".to_string()],
        )
        .unwrap();

        assert_eq!(manifest.chunk_count, 4);
        assert!(manifest.is_encrypted);
        assert_eq!(manifest.chunks.len(), 4);
        assert!(chunks_dir.join("manifest.json").exists());
        assert!(chunks_dir.join("chunk_0000.bin").exists());
    }
}
