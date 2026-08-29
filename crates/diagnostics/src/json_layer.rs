use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use regex::Regex;
use std::sync::OnceLock;

/// Standardized JSON log record schema conforming to spec §57.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredLogRecord {
    pub timestamp: DateTime<Utc>,
    pub process: String,
    pub machine_id: String,
    pub session_id: Option<String>,
    pub module: String,
    pub severity: String,
    pub event: String,
    pub message: String,
    pub error_code: Option<String>,
    pub duration_us: Option<u64>,
    pub metadata: serde_json::Value,
}

static JSON_KV_REGEX: OnceLock<Regex> = OnceLock::new();
static KV_PAIR_REGEX: OnceLock<Regex> = OnceLock::new();
static COLON_PAIR_REGEX: OnceLock<Regex> = OnceLock::new();
static BEARER_REGEX: OnceLock<Regex> = OnceLock::new();
static SSN_REGEX: OnceLock<Regex> = OnceLock::new();
static CREDIT_CARD_REGEX: OnceLock<Regex> = OnceLock::new();

fn get_json_kv_regex() -> &'static Regex {
    JSON_KV_REGEX.get_or_init(|| {
        Regex::new(r#"(?i)"(password|secret|token|api_key|apikey|access_token|refresh_token|private_key|auth_token|client_secret)"\s*:\s*"[^"]*""#)
            .expect("Valid JSON KV regex")
    })
}

fn get_kv_pair_regex() -> &'static Regex {
    KV_PAIR_REGEX.get_or_init(|| {
        Regex::new(r#"(?i)\b(password|secret|token|api_key|apikey|access_token|refresh_token|private_key|auth_token|client_secret|authorization|bearer)\s*=\s*['"]?[^\s,&;'"\}\]]+['"]?"#)
            .expect("Valid KV pair regex")
    })
}

fn get_colon_pair_regex() -> &'static Regex {
    COLON_PAIR_REGEX.get_or_init(|| {
        Regex::new(r#"(?i)\b(password|secret|token|api_key|apikey|access_token|refresh_token|private_key|auth_token|client_secret)\s*:\s*['"]?[^\s,;'"\}\]\[]+['"]?"#)
            .expect("Valid colon pair regex")
    })
}

fn get_bearer_regex() -> &'static Regex {
    BEARER_REGEX.get_or_init(|| {
        Regex::new(r#"(?i)\b(bearer|basic)\s+[a-zA-Z0-9_\-\.~+/]{6,}=*"#)
            .expect("Valid Bearer regex")
    })
}

fn get_ssn_regex() -> &'static Regex {
    SSN_REGEX.get_or_init(|| {
        Regex::new(r#"\b\d{3}-\d{2}-\d{4}\b"#)
            .expect("Valid SSN regex")
    })
}

fn get_credit_card_regex() -> &'static Regex {
    CREDIT_CARD_REGEX.get_or_init(|| {
        Regex::new(r#"\b(?:\d{4}[ -]?){3}\d{4}\b"#)
            .expect("Valid Credit Card regex")
    })
}

pub struct JsonPrivacyFormatter;

impl JsonPrivacyFormatter {
    /// Sanitize log messages and field values to ensure no raw passwords/PII leak to logs.
    pub fn sanitize_message(raw_msg: &str) -> String {
        if raw_msg.is_empty() {
            return String::new();
        }

        // 1. JSON colon key-values: e.g. "password": "secret" -> "password": "[REDACTED]"
        let sanitized = get_json_kv_regex()
            .replace_all(raw_msg, r#""$1": "[REDACTED]""#);

        // 2. Key=Value & query string pairs: e.g. password=SuperSecret or ?api_key=123 -> $1=[REDACTED]
        let sanitized = get_kv_pair_regex()
            .replace_all(&sanitized, "$1=[REDACTED]");

        // 3. Colon key-value pairs (unquoted/YAML): e.g. password: secret -> $1: [REDACTED]
        let sanitized = get_colon_pair_regex()
            .replace_all(&sanitized, "$1: [REDACTED]");

        // 4. Bearer & Basic Authorization headers/tokens: e.g. Bearer eyJhbGci... -> Bearer [REDACTED]
        let sanitized = get_bearer_regex()
            .replace_all(&sanitized, "$1 [REDACTED]");

        // 5. Social Security Numbers (SSN): e.g. 123-45-6789 -> [REDACTED_SSN]
        let sanitized = get_ssn_regex()
            .replace_all(&sanitized, "[REDACTED_SSN]");

        // 6. Credit Card numbers: e.g. 4532-1234-5678-9012 -> [REDACTED_CARD]
        let sanitized = get_credit_card_regex()
            .replace_all(&sanitized, "[REDACTED_CARD]");

        sanitized.into_owned()
    }

    /// Recursively sanitize all string fields in a JSON value.
    pub fn sanitize_value(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::String(s) => {
                *s = Self::sanitize_message(s);
            }
            serde_json::Value::Array(arr) => {
                for item in arr {
                    Self::sanitize_value(item);
                }
            }
            serde_json::Value::Object(map) => {
                for (k, v) in map.iter_mut() {
                    let k_lower = k.to_ascii_lowercase();
                    if k_lower.contains("password")
                        || k_lower.contains("secret")
                        || k_lower.contains("token")
                        || k_lower.contains("api_key")
                        || k_lower.contains("private_key")
                    {
                        *v = serde_json::Value::String("[REDACTED]".to_string());
                    } else {
                        Self::sanitize_value(v);
                    }
                }
            }
            _ => {}
        }
    }

    /// Sanitize an entire structured log record in-place.
    pub fn sanitize_record(record: &mut StructuredLogRecord) {
        record.message = Self::sanitize_message(&record.message);
        Self::sanitize_value(&mut record.metadata);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_privacy_log_formatter_key_value_redaction() {
        let msg = "User authentication failed password=SuperSecretPassword123 with status 401";
        let sanitized = JsonPrivacyFormatter::sanitize_message(msg);
        assert_eq!(
            sanitized,
            "User authentication failed password=[REDACTED] with status 401"
        );

        let msg2 = "Connection failed api_key='sk-test-secret-key-123' and secret=\"hidden_val\"";
        let sanitized2 = JsonPrivacyFormatter::sanitize_message(msg2);
        assert_eq!(
            sanitized2,
            "Connection failed api_key=[REDACTED] and secret=[REDACTED]"
        );
    }

    #[test]
    fn test_privacy_log_formatter_bearer_token_redaction() {
        let msg = "HTTP Request: Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.token123 to /api";
        let sanitized = JsonPrivacyFormatter::sanitize_message(msg);
        assert_eq!(
            sanitized,
            "HTTP Request: Authorization: Bearer [REDACTED] to /api"
        );

        let msg2 = "Token used: bearer 1234567890abcdef1234 in header";
        let sanitized2 = JsonPrivacyFormatter::sanitize_message(msg2);
        assert_eq!(
            sanitized2,
            "Token used: bearer [REDACTED] in header"
        );

        let msg3 = "Basic auth: Authorization: Basic dXNlcjpwYXNz";
        let sanitized3 = JsonPrivacyFormatter::sanitize_message(msg3);
        assert_eq!(
            sanitized3,
            "Basic auth: Authorization: Basic [REDACTED]"
        );
    }

    #[test]
    fn test_privacy_log_formatter_json_colon_redaction() {
        let msg = r#"Payload received: {"user": "alice", "password": "SuperSecretPassword123", "api_key": "sk-999"}"#;
        let sanitized = JsonPrivacyFormatter::sanitize_message(msg);
        assert_eq!(
            sanitized,
            r#"Payload received: {"user": "alice", "password": "[REDACTED]", "api_key": "[REDACTED]"}"#
        );

        let msg2 = r#"Key dump: {"private_key": "-----BEGIN RSA PRIVATE KEY-----..."}"#;
        let sanitized2 = JsonPrivacyFormatter::sanitize_message(msg2);
        assert_eq!(
            sanitized2,
            r#"Key dump: {"private_key": "[REDACTED]"}"#
        );
    }

    #[test]
    fn test_privacy_log_formatter_ssn_redaction() {
        let msg = "Applicant background check for SSN: 123-45-6789 verified";
        let sanitized = JsonPrivacyFormatter::sanitize_message(msg);
        assert_eq!(
            sanitized,
            "Applicant background check for SSN: [REDACTED_SSN] verified"
        );

        let msg2 = "SSN 000-12-3456 processed";
        let sanitized2 = JsonPrivacyFormatter::sanitize_message(msg2);
        assert_eq!(
            sanitized2,
            "SSN [REDACTED_SSN] processed"
        );
    }

    #[test]
    fn test_privacy_log_formatter_credit_card_redaction() {
        let msg = "Payment processed with card 4532-1234-5678-9012 successfully";
        let sanitized = JsonPrivacyFormatter::sanitize_message(msg);
        assert_eq!(
            sanitized,
            "Payment processed with card [REDACTED_CARD] successfully"
        );

        let msg2 = "Card with spaces: 4532 1234 5678 9012";
        let sanitized2 = JsonPrivacyFormatter::sanitize_message(msg2);
        assert_eq!(
            sanitized2,
            "Card with spaces: [REDACTED_CARD]"
        );

        let msg3 = "Card contiguous: 4532123456789012";
        let sanitized3 = JsonPrivacyFormatter::sanitize_message(msg3);
        assert_eq!(
            sanitized3,
            "Card contiguous: [REDACTED_CARD]"
        );
    }

    #[test]
    fn test_privacy_log_formatter_query_string_redaction() {
        let url = "GET /api/v1/data?token=abcde12345&secret=xyz999&limit=50";
        let sanitized = JsonPrivacyFormatter::sanitize_message(url);
        assert_eq!(
            sanitized,
            "GET /api/v1/data?token=[REDACTED]&secret=[REDACTED]&limit=50"
        );
    }

    #[test]
    fn test_privacy_log_formatter_unquoted_colon_redaction() {
        let msg = "Config: password: mySecretPassword123, secret: tokenVal";
        let sanitized = JsonPrivacyFormatter::sanitize_message(msg);
        assert_eq!(
            sanitized,
            "Config: password: [REDACTED], secret: [REDACTED]"
        );
    }

    #[test]
    fn test_privacy_log_formatter_safe_messages() {
        let safe_msg = "User authentication failed for user john_doe";
        assert_eq!(JsonPrivacyFormatter::sanitize_message(safe_msg), safe_msg);

        let empty = "";
        assert_eq!(JsonPrivacyFormatter::sanitize_message(empty), "");
    }

    #[test]
    fn test_privacy_log_formatter_sanitize_record_and_value() {
        let mut record = StructuredLogRecord {
            timestamp: Utc::now(),
            process: "agent".to_string(),
            machine_id: "m1".to_string(),
            session_id: Some("s1".to_string()),
            module: "auth".to_string(),
            severity: "WARN".to_string(),
            event: "LOGIN_FAILED".to_string(),
            message: "Failed password=badpass for user".to_string(),
            error_code: Some("AUTH_01".to_string()),
            duration_us: Some(100),
            metadata: serde_json::json!({
                "api_key": "sk-123456",
                "nested": {
                    "token": "tok-789",
                    "details": "SSN is 123-45-6789"
                }
            }),
        };

        JsonPrivacyFormatter::sanitize_record(&mut record);
        assert_eq!(record.message, "Failed password=[REDACTED] for user");
        assert_eq!(record.metadata["api_key"], "[REDACTED]");
        assert_eq!(record.metadata["nested"]["token"], "[REDACTED]");
        assert_eq!(record.metadata["nested"]["details"], "SSN is [REDACTED_SSN]");
    }
}
