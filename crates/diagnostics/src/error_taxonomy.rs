use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Transient,
    Recoverable,
    Degraded,
    Fatal,
    PrivacyCritical,
}

#[derive(Debug, Error)]
pub enum DiagnosticsError {
    #[error("Diagnostics initialization failed: {0}")]
    InitError(String),
    #[error("I/O error encountered in logging: {0}")]
    IoError(String),
    #[error("Probe evaluation error: {0}")]
    ProbeError(String),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticEvent {
    pub schema: String, // "gtf.trajectory.diagnostic"
    pub schema_version: String, // "1.0"
    pub event_id: u64,
    pub timestamp_utc: chrono::DateTime<chrono::Utc>,
    pub severity: DiagnosticSeverity,
    pub subsystem: String,
    pub error_code: String,
    pub description: String,
    pub remediation_action: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_taxonomy_serialization() {
        let event = DiagnosticEvent {
            schema: "gtf.trajectory.diagnostic".to_string(),
            schema_version: "1.0".to_string(),
            event_id: 1,
            timestamp_utc: chrono::Utc::now(),
            severity: DiagnosticSeverity::Degraded,
            subsystem: "video-capture".to_string(),
            error_code: "DDA_FRAME_TIMEOUT".to_string(),
            description: "DDA capture timed out, fallback to GDI".to_string(),
            remediation_action: Some("Check display driver".to_string()),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("gtf.trajectory.diagnostic"));
        assert!(json.contains("DEGRADED"));
    }
}
