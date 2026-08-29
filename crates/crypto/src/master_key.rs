use rand::RngCore;
use zeroize::{Zeroize, ZeroizeOnDrop};
use crate::dpapi::Dpapi;
use crate::error::CryptoError;

pub const KEY_SIZE_BYTES: usize = 32; // 256 bits

#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct MasterKey {
    bytes: [u8; KEY_SIZE_BYTES],
}

impl MasterKey {
    /// Generates a cryptographically secure random 256-bit key using OsRng.
    pub fn generate() -> Self {
        let mut bytes = [0u8; KEY_SIZE_BYTES];
        rand::rngs::OsRng.fill_bytes(&mut bytes);
        Self { bytes }
    }

    pub fn from_bytes(bytes: [u8; KEY_SIZE_BYTES]) -> Self {
        Self { bytes }
    }

    pub fn as_bytes(&self) -> &[u8; KEY_SIZE_BYTES] {
        &self.bytes
    }

    /// Protects the master key using machine-level DPAPI for local disk storage.
    pub fn save_dpapi_protected(&self, entropy_salt: Option<&[u8]>) -> Result<Vec<u8>, CryptoError> {
        Dpapi::protect_machine_secret(&self.bytes, entropy_salt)
    }

    /// Loads and unprotects a DPAPI-encrypted master key from disk.
    pub fn load_dpapi_protected(ciphertext: &[u8], entropy_salt: Option<&[u8]>) -> Result<Self, CryptoError> {
        let mut raw = Dpapi::unprotect(ciphertext, entropy_salt)?;
        if raw.len() != KEY_SIZE_BYTES {
            raw.zeroize();
            return Err(CryptoError::InvalidKeyLength {
                expected: KEY_SIZE_BYTES,
                actual: raw.len(),
            });
        }
        let mut key_bytes = [0u8; KEY_SIZE_BYTES];
        key_bytes.copy_from_slice(&raw);
        raw.zeroize();
        Ok(Self { bytes: key_bytes })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_master_key_generation_and_zeroize() {
        let key = MasterKey::generate();
        assert_eq!(key.as_bytes().len(), KEY_SIZE_BYTES);
        assert_ne!(key.as_bytes(), &[0u8; KEY_SIZE_BYTES]);
    }

    #[test]
    fn test_master_key_dpapi_roundtrip() {
        let key = MasterKey::generate();
        let protected = key.save_dpapi_protected(None).unwrap();
        let loaded = MasterKey::load_dpapi_protected(&protected, None).unwrap();
        assert_eq!(key.as_bytes(), loaded.as_bytes());
    }
}
