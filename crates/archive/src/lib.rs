//! Streaming TAR + Zstandard packaging, XChaCha20-Poly1305 encryption, and chunking.

pub mod chunker;
pub mod compress;
pub mod manifest;

pub use chunker::chunk_and_encrypt_archive;
pub use compress::{create_tar_zstd_archive, extract_tar_zstd_archive};
pub use manifest::{ArchiveChunkEntry, SessionArchiveManifest};

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_full_archive_and_chunk_pipeline() {
        let dir = tempdir().unwrap();
        let session_dir = dir.path().join("session_1");
        let archive_file = dir.path().join("archive.tar.zst");
        let chunks_dir = dir.path().join("chunks");

        std::fs::create_dir_all(&session_dir).unwrap();
        std::fs::write(session_dir.join("a.ndjson"), "line1\nline2\n").unwrap();
        std::fs::write(session_dir.join("b.db"), "data").unwrap();

        let (uncompressed, _compressed, files) = create_tar_zstd_archive(&session_dir, &archive_file, 3).unwrap();
        assert_eq!(files.len(), 2);

        let manifest = chunk_and_encrypt_archive(
            &archive_file,
            &chunks_dir,
            "SESS_FULL",
            1024, // 1KB chunks
            None,
            uncompressed,
            files,
        )
        .unwrap();

        assert!(manifest.chunk_count >= 1);
        assert!(!manifest.is_encrypted);
    }
}
