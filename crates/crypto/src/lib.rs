//! Cryptographic primitives: Windows DPAPI wrappers, XChaCha20-Poly1305 AEAD, and SHA-256 digests.

pub mod aead;
pub mod dpapi;
pub mod error;
pub mod hash;
pub mod master_key;

pub use aead::{XChaCha20Aead, MAGIC_ENCRYPTED_HEADER, NONCE_SIZE_BYTES, TAG_SIZE_BYTES};
pub use dpapi::Dpapi;
pub use error::CryptoError;
pub use hash::{
    compute_sha256, compute_sha256_hex, hash_file_sha256, verify_sha256_hex, Sha256Hasher,
};
pub use master_key::{MasterKey, KEY_SIZE_BYTES};
