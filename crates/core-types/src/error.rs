use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Classification of operational error severity for health alerting and degradation control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorSeverity {
    /// Transient glitch, automatically retried (e.g. temporary network blip).
    Transient,
    /// Non-fatal error, subsystem recovered (e.g. UIA timeout fallback to coordinate).
    Recoverable,
    /// Subsystem disabled, overall recorder continues (e.g. video capture dropped due to disk pressure).
    Degraded,
    /// Critical unrecoverable error, process must cleanly finalize and stop (e.g. missing encryption key).
    Fatal,
    /// Privacy filter uncertain, must fail-closed (no plaintext persisted).
    PrivacyCritical,
}

/// Unified error taxonomy across all recorder subsystems.
#[derive(Error, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ErrorTaxonomy {
    #[error("Capture subsystem error [{severity:?}]: {message}")]
    Capture {
        severity: ErrorSeverity,
        message: String,
    },

    #[error("UI Automation error [{severity:?}]: {message}")]
    UiAutomation {
        severity: ErrorSeverity,
        message: String,
    },

    #[error("Privacy engine violation [{severity:?}]: {message}")]
    Privacy {
        severity: ErrorSeverity,
        message: String,
    },

    #[error("Spool/Storage error [{severity:?}]: {message}")]
    Spool {
        severity: ErrorSeverity,
        message: String,
    },

    #[error("Archive compression/packaging error [{severity:?}]: {message}")]
    Archive {
        severity: ErrorSeverity,
        message: String,
    },

    #[error("Cryptography error [{severity:?}]: {message}")]
    Crypto {
        severity: ErrorSeverity,
        message: String,
    },

    #[error("Upload client error [{severity:?}]: {message}")]
    Upload {
        severity: ErrorSeverity,
        message: String,
    },

    #[error("Ingestion server error [{severity:?}]: {message}")]
    Server {
        severity: ErrorSeverity,
        message: String,
    },

    #[error("IPC communication error [{severity:?}]: {message}")]
    Ipc {
        severity: ErrorSeverity,
        message: String,
    },

    #[error("Configuration error [{severity:?}]: {message}")]
    Config {
        severity: ErrorSeverity,
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_taxonomy_formatting_and_serde() {
        let err = ErrorTaxonomy::Capture {
            severity: ErrorSeverity::Transient,
            message: "Hook timeout".to_string(),
        };

        assert_eq!(
            err.to_string(),
            "Capture subsystem error [Transient]: Hook timeout"
        );

        let json = serde_json::to_string(&err).unwrap();
        let deserialized: ErrorTaxonomy = serde_json::from_str(&json).unwrap();
        assert_eq!(err, deserialized);
    }
}
