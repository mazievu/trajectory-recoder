use sha2::{Digest, Sha256};

/// Compute SHA-256 hex digest of a byte slice.
pub fn compute_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let result = hasher.finalize();
    hex::encode(result)
}

/// Compute SHA-256 hex digest of a string.
pub fn compute_sha256_str(text: &str) -> String {
    compute_sha256(text.as_bytes())
}
