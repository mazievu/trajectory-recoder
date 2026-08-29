use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum IpcError {
    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    #[error("Frame size {size} exceeds maximum limit of {max} bytes")]
    FrameTooLarge { size: usize, max: usize },

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Pipe creation error: {0}")]
    PipeCreationError(String),

    #[error("Security descriptor error: {0}")]
    SecurityDescriptorError(String),

    #[error("Platform unsupported for native operation")]
    UnsupportedPlatform,

    #[error("IO error: {0}")]
    Io(String),
}

impl From<std::io::Error> for IpcError {
    fn from(err: std::io::Error) -> Self {
        IpcError::Io(err.to_string())
    }
}
