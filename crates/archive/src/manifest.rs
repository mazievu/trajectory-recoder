use serde::{Deserialize, Serialize};

/// Session packaging manifest describing uncompressed files and packaging metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionArchiveManifest {
    pub session_id: String,
    pub created_at_utc: String,
    pub uncompressed_size_bytes: u64,
    pub compressed_size_bytes: u64,
    pub is_encrypted: bool,
    pub archive_sha256: String,
    pub chunk_count: usize,
    pub chunk_size_bytes: usize,
    pub chunks: Vec<ArchiveChunkEntry>,
    pub file_list: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArchiveChunkEntry {
    pub chunk_index: usize,
    pub file_name: String,
    pub byte_size: usize,
    pub sha256: String,
}
