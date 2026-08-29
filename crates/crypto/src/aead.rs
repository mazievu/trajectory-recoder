use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    XChaCha20Poly1305, XNonce,
};
use rand::RngCore;
use crate::error::CryptoError;
use crate::master_key::MasterKey;

pub const NONCE_SIZE_BYTES: usize = 24; // 192 bits
pub const TAG_SIZE_BYTES: usize = 16;   // 128 bits
pub const MAGIC_ENCRYPTED_HEADER: &[u8; 4] = b"TREC"; // Trajectory Recorder Encrypted Chunk

pub struct XChaCha20Aead;

impl XChaCha20Aead {
    /// Encrypts plaintext with XChaCha20-Poly1305.
    /// Output layout: [24-byte Nonce || Ciphertext + 16-byte Poly1305 Tag]
    pub fn encrypt(
        key: &MasterKey,
        plaintext: &[u8],
        associated_data: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        let cipher = XChaCha20Poly1305::new_from_slice(key.as_bytes())
            .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;

        let mut nonce_bytes = [0u8; NONCE_SIZE_BYTES];
        rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = XNonce::from_slice(&nonce_bytes);

        let payload = Payload {
            msg: plaintext,
            aad: associated_data,
        };

        let ciphertext = cipher
            .encrypt(nonce, payload)
            .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;

        let mut output = Vec::with_capacity(NONCE_SIZE_BYTES + ciphertext.len());
        output.extend_from_slice(&nonce_bytes);
        output.extend_from_slice(&ciphertext);
        Ok(output)
    }

    /// Decrypts payload layout: [24-byte Nonce || Ciphertext + 16-byte Poly1305 Tag]
    pub fn decrypt(
        key: &MasterKey,
        payload_bytes: &[u8],
        associated_data: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        if payload_bytes.len() < NONCE_SIZE_BYTES + TAG_SIZE_BYTES {
            return Err(CryptoError::InvalidPayloadLength {
                actual: payload_bytes.len(),
            });
        }

        let cipher = XChaCha20Poly1305::new_from_slice(key.as_bytes())
            .map_err(|e| CryptoError::DecryptionFailed(e.to_string()))?;

        let (nonce_bytes, ciphertext) = payload_bytes.split_at(NONCE_SIZE_BYTES);
        let nonce = XNonce::from_slice(nonce_bytes);

        let payload = Payload {
            msg: ciphertext,
            aad: associated_data,
        };

        cipher
            .decrypt(nonce, payload)
            .map_err(|_| CryptoError::DecryptionFailed("Authentication tag verification failed or ciphertext corrupted".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xchacha20_aead_roundtrip() {
        let key = MasterKey::generate();
        let plaintext = b"Hello world! This is a test of XChaCha20-Poly1305 authenticated encryption.";
        let aad = b"session_id=test_123_chunk_0";

        let encrypted = XChaCha20Aead::encrypt(&key, plaintext, aad).unwrap();
        assert_eq!(encrypted.len(), NONCE_SIZE_BYTES + plaintext.len() + TAG_SIZE_BYTES);

        let decrypted = XChaCha20Aead::decrypt(&key, &encrypted, aad).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_xchacha20_aead_tamper_detection() {
        let key = MasterKey::generate();
        let plaintext = b"Sensitive payload";
        let aad = b"context_aad";

        let mut encrypted = XChaCha20Aead::encrypt(&key, plaintext, aad).unwrap();

        // Tamper with ciphertext byte
        let last_idx = encrypted.len() - 1;
        encrypted[last_idx] ^= 0x01;

        let result = XChaCha20Aead::decrypt(&key, &encrypted, aad);
        assert!(result.is_err());

        // Restore byte, tamper with AAD
        encrypted[last_idx] ^= 0x01;
        let tampered_aad = b"wrong_aad";
        let result_aad = XChaCha20Aead::decrypt(&key, &encrypted, tampered_aad);
        assert!(result_aad.is_err());
    }
}
