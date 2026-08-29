use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use std::io::Read;
use std::path::Path;

pub struct Sha256Hasher {
    hasher: Sha256,
}

impl Default for Sha256Hasher {
    fn default() -> Self {
        Self { hasher: Sha256::new() }
    }
}

impl Sha256Hasher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, data: &[u8]) {
        self.hasher.update(data);
    }

    pub fn finalize(self) -> [u8; 32] {
        self.hasher.finalize().into()
    }

    pub fn finalize_hex(self) -> String {
        hex::encode(self.finalize())
    }
}

/// Computes raw 32-byte SHA-256 digest of in-memory data
pub fn compute_sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Computes lowercase hex-encoded SHA-256 string
pub fn compute_sha256_hex(data: &[u8]) -> String {
    hex::encode(compute_sha256(data))
}

/// Streams a file from disk in 64 KiB chunks and computes its SHA-256 digest
pub fn hash_file_sha256(path: &Path) -> Result<String, std::io::Error> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536]; // 64 KiB buffer

    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }

    Ok(hex::encode(hasher.finalize()))
}

/// Constant-time verification of computed vs expected SHA-256 hex string
pub fn verify_sha256_hex(computed_hex: &str, expected_hex: &str) -> bool {
    if computed_hex.len() != expected_hex.len() {
        return false;
    }
    computed_hex.as_bytes().ct_eq(expected_hex.as_bytes()).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_empty_string_vector() {
        let hash = compute_sha256_hex(b"");
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert!(verify_sha256_hex(&hash, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"));
        assert!(!verify_sha256_hex(&hash, "0000000000000000000000000000000000000000000000000000000000000000"));
    }

    #[test]
    fn test_sha256_streaming() {
        let mut hasher = Sha256Hasher::new();
        hasher.update(b"hello ");
        hasher.update(b"world");
        let hex = hasher.finalize_hex();
        assert_eq!(hex, compute_sha256_hex(b"hello world"));
    }
}
