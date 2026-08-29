//! In-memory regex, Shannon entropy, and fail-closed privacy redaction engine.

pub mod engine;
pub mod entropy;
pub mod patterns;

pub use engine::{PrivacyEngine, PrivacyPolicy};
pub use entropy::{calculate_shannon_entropy, is_high_entropy_secret};
pub use patterns::is_valid_luhn_credit_card;

#[cfg(test)]
mod tests {
    use super::*;
    use core_types::action::{ActionParameters, CanonicalActionBuilder, TypeTextParams};
    use core_types::id::{GlobalEventId, SessionId};
    use core_types::metadata::TargetMetadata;
    use core_types::timestamp::DualTimestamp;

    #[test]
    fn test_tier1_password_box_redaction() {
        let engine = PrivacyEngine::default();
        let mut target = TargetMetadata {
            is_password: true,
            value: Some("MySecretP@ssword".to_string()),
            ..Default::default()
        };

        engine.redact_target_metadata(&mut target);
        assert_eq!(target.value.as_deref(), Some("[PASSWORD_REDACTED]"));
    }

    #[test]
    fn test_tier2_ssn_and_credit_card_redaction() {
        let engine = PrivacyEngine::default();

        let raw_text = "Client SSN is 123-45-6789 and Card is 4532-0151-1283-0366.";
        let (redacted, changed) = engine.redact_text(raw_text);

        assert!(changed);
        assert!(redacted.contains("[SSN_REDACTED]"));
        assert!(redacted.contains("[CREDIT_CARD_REDACTED]"));
        assert!(!redacted.contains("123-45-6789"));
        assert!(!redacted.contains("4532-0151-1283-0366"));
    }

    #[test]
    fn test_tier2_api_key_redaction() {
        let engine = PrivacyEngine::default();

        let text = "Deploy using AKIAIOSFODNN7EXAMPLE key and Bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.do_not_leak";
        let (redacted, changed) = engine.redact_text(text);

        assert!(changed);
        assert!(redacted.contains("[API_KEY_REDACTED]"));
        assert!(!redacted.contains("AKIAIOSFODNN7EXAMPLE"));
    }

    #[test]
    fn test_tier3_high_entropy_token_redaction() {
        let engine = PrivacyEngine::default();

        let text = "Token is 9xK8#mP2!qR5vL7*wZ1&yN4^jB6~hT3@ strictly secret.";
        let (redacted, changed) = engine.redact_text(text);

        assert!(changed);
        assert!(redacted.contains("[HIGH_ENTROPY_REDACTED]"));
        assert!(!redacted.contains("9xK8#mP2!qR5vL7*wZ1&yN4^jB6~hT3@"));
    }

    #[test]
    fn test_canonical_text_is_fail_closed_when_target_safety_is_unknown() {
        let engine = PrivacyEngine::default();

        let mut action = CanonicalActionBuilder::new(
            GlobalEventId::new(1),
            SessionId::new("sess_1"),
            1,
            DualTimestamp::now(),
            core_types::action::ActionType::TypeText,
            ActionParameters::TypeText(TypeTextParams {
                text: "ordinary typed text".to_string(),
                length: 19,
                is_redacted: false,
                character_count: 15,
                backspace_count: 0,
                enter_pressed: true,
            }),
        )
        .build();

        engine.redact_canonical_action(&mut action);

        if let ActionParameters::TypeText(ref tp) = action.parameters {
            assert!(tp.is_redacted);
            assert_eq!(tp.text, "[UNOBSERVED_TEXT]");
        } else {
            panic!("Expected TypeText parameters");
        }
    }
}
