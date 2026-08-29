use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use rand::RngCore;
use sha2::{Digest, Sha256};

pub struct CryptoVerifier;

impl CryptoVerifier {
    pub const KEY_SIZE: usize = 32; // 256-bit key
    pub const NONCE_SIZE: usize = 24; // 192-bit nonce for XChaCha20

    pub fn generate_key() -> [u8; Self::KEY_SIZE] {
        let mut key = [0u8; Self::KEY_SIZE];
        rand::thread_rng().fill_bytes(&mut key);
        key
    }

    pub fn generate_nonce() -> [u8; Self::NONCE_SIZE] {
        let mut nonce = [0u8; Self::NONCE_SIZE];
        rand::thread_rng().fill_bytes(&mut nonce);
        nonce
    }

    pub fn encrypt_payload(key: &[u8; 32], nonce_bytes: &[u8; 24], plaintext: &[u8]) -> Result<Vec<u8>, String> {
        let cipher = XChaCha20Poly1305::new(key.into());
        let nonce = XNonce::from_slice(nonce_bytes);
        cipher.encrypt(nonce, plaintext).map_err(|e| format!("Encryption error: {:?}", e))
    }

    pub fn decrypt_payload(key: &[u8; 32], nonce_bytes: &[u8; 24], ciphertext: &[u8]) -> Result<Vec<u8>, String> {
        let cipher = XChaCha20Poly1305::new(key.into());
        let nonce = XNonce::from_slice(nonce_bytes);
        cipher.decrypt(nonce, ciphertext).map_err(|e| format!("Decryption error / authentication failed: {:?}", e))
    }

    pub fn compute_sha256_hex(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    pub fn verify_tamper_rejection(key: &[u8; 32], nonce: &[u8; 24], original_plaintext: &[u8]) -> Result<(), String> {
        let mut ciphertext = Self::encrypt_payload(key, nonce, original_plaintext)?;
        if ciphertext.is_empty() {
            return Err("Ciphertext unexpectedly empty".to_string());
        }

        // Tamper with 1 byte
        ciphertext[0] ^= 0xFF;

        match Self::decrypt_payload(key, nonce, &ciphertext) {
            Ok(_) => Err("Tampered ciphertext succeeded decryption! AEAD authentication failed to catch modification.".to_string()),
            Err(_) => Ok(()), // Correctly rejected
        }
    }
}
