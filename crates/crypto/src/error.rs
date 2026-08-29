use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum CryptoError {
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("Invalid payload length: actual {actual}")]
    InvalidPayloadLength { actual: usize },

    #[error("Invalid key length: expected {expected}, actual {actual}")]
    InvalidKeyLength { expected: usize, actual: usize },

    #[error("DPAPI protect failed (code {error_code}): {message}")]
    DpapiProtectFailed { error_code: u32, message: String },

    #[error("DPAPI unprotect failed (code {error_code}): {message}")]
    DpapiUnprotectFailed { error_code: u32, message: String },

    #[error("Platform unsupported for native operation")]
    UnsupportedPlatform,

    #[error("IO error: {0}")]
    Io(String),
}

impl From<std::io::Error> for CryptoError {
    fn from(err: std::io::Error) -> Self {
        CryptoError::Io(err.to_string())
    }
}
