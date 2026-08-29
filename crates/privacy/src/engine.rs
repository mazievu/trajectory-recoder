use crate::entropy::is_high_entropy_secret;
use crate::patterns::{
    API_KEY_REGEX, CC_CANDIDATE_REGEX, EMAIL_REGEX, SSN_REGEX, URL_PASSWORD_REGEX,
    is_valid_luhn_credit_card,
};
use core_types::action::{ActionParameters, CanonicalAction};
use core_types::event::{RawEvent, RawEventPayload};
use core_types::metadata::TargetMetadata;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyPolicy {
    pub mask_passwords: bool,
    pub mask_ssn: bool,
    pub mask_credit_cards: bool,
    pub mask_api_keys: bool,
    pub mask_high_entropy: bool,
    pub entropy_threshold: f64,
    pub min_entropy_len: usize,
    pub mask_emails: bool,
    pub custom_regexes: Vec<String>,
}

impl Default for PrivacyPolicy {
    fn default() -> Self {
        Self {
            mask_passwords: true,
            mask_ssn: true,
            mask_credit_cards: true,
            mask_api_keys: true,
            mask_high_entropy: true,
            entropy_threshold: 4.5,
            min_entropy_len: 16,
            mask_emails: false,
            custom_regexes: Vec::new(),
        }
    }
}

/// In-memory 3-tier privacy engine enforcing fail-closed redaction before persistence or IPC.
#[derive(Clone)]
pub struct PrivacyEngine {
    policy: PrivacyPolicy,
    compiled_custom_regexes: Vec<Regex>,
}

impl Default for PrivacyEngine {
    fn default() -> Self {
        Self::new(PrivacyPolicy::default())
    }
}

impl PrivacyEngine {
    pub fn new(policy: PrivacyPolicy) -> Self {
        let mut compiled = Vec::new();
        for r_str in &policy.custom_regexes {
            if let Ok(reg) = Regex::new(r_str) {
                compiled.push(reg);
            }
        }
        Self {
            policy,
            compiled_custom_regexes: compiled,
        }
    }

    /// Redact a raw string according to configured privacy tiers.
    pub fn redact_text(&self, text: &str) -> (String, bool) {
        if text.is_empty() {
            return (String::new(), false);
        }

        let mut redacted = text.to_string();
        let mut changed = false;

        // URL passwords
        if URL_PASSWORD_REGEX.is_match(&redacted) {
            redacted = URL_PASSWORD_REGEX
                .replace_all(&redacted, "https://$1:[PASSWORD_REDACTED]@")
                .to_string();
            changed = true;
        }

        // Tier 2: SSN
        if self.policy.mask_ssn && SSN_REGEX.is_match(&redacted) {
            redacted = SSN_REGEX
                .replace_all(&redacted, "[SSN_REDACTED]")
                .to_string();
            changed = true;
        }

        // Tier 2: API Keys
        if self.policy.mask_api_keys && API_KEY_REGEX.is_match(&redacted) {
            redacted = API_KEY_REGEX
                .replace_all(&redacted, "[API_KEY_REDACTED]")
                .to_string();
            changed = true;
        }

        // Tier 2: Credit Cards with Luhn validation
        if self.policy.mask_credit_cards {
            let mut replacements = Vec::new();
            for mat in CC_CANDIDATE_REGEX.find_iter(&redacted) {
                let candidate = mat.as_str();
                if is_valid_luhn_credit_card(candidate) {
                    replacements.push((mat.start(), mat.end(), "[CREDIT_CARD_REDACTED]"));
                }
            }
            if !replacements.is_empty() {
                let mut result = String::new();
                let mut last_end = 0;
                for (start, end, repl) in replacements {
                    if start >= last_end {
                        result.push_str(&redacted[last_end..start]);
                        result.push_str(repl);
                        last_end = end;
                    }
                }
                result.push_str(&redacted[last_end..]);
                redacted = result;
                changed = true;
            }
        }

        // Optional: Emails
        if self.policy.mask_emails && EMAIL_REGEX.is_match(&redacted) {
            redacted = EMAIL_REGEX
                .replace_all(&redacted, "[EMAIL_REDACTED]")
                .to_string();
            changed = true;
        }

        // Custom regexes
        for reg in &self.compiled_custom_regexes {
            if reg.is_match(&redacted) {
                redacted = reg.replace_all(&redacted, "[REDACTED]").to_string();
                changed = true;
            }
        }

        // Tier 3: Shannon Entropy
        if self.policy.mask_high_entropy {
            let words: Vec<&str> = redacted.split_whitespace().collect();
            let mut word_replacements = Vec::new();
            for word in words {
                if is_high_entropy_secret(
                    word,
                    self.policy.entropy_threshold,
                    self.policy.min_entropy_len,
                ) {
                    word_replacements.push((word.to_string(), "[HIGH_ENTROPY_REDACTED]"));
                }
            }
            for (w, r) in word_replacements {
                redacted = redacted.replace(&w, r);
                changed = true;
            }
        }

        (redacted, changed)
    }

    /// Redact a target UI element metadata in place (e.g. password box text or confidential values).
    pub fn redact_target_metadata(&self, target: &mut TargetMetadata) {
        if target.is_password {
            target.value = Some("[PASSWORD_REDACTED]".to_string());
        } else if let Some(ref val) = target.value {
            let (clean, _) = self.redact_text(val);
            target.value = Some(clean);
        }

        if let Some(ref name) = target.name {
            let (clean, _) = self.redact_text(name);
            target.name = Some(clean);
        }
    }

    /// Redact a `CanonicalAction` before saving to disk or sending over network.
    pub fn redact_canonical_action(&self, action: &mut CanonicalAction) {
        // Redact target metadata
        self.redact_target_metadata(&mut action.target);

        // Redact parameters
        match &mut action.parameters {
            ActionParameters::TypeText(tp) => {
                if action.target.is_password {
                    tp.text = "[PASSWORD_REDACTED]".to_string();
                } else {
                    // A Win32 hook cannot reliably establish that a text
                    // field is safe. Persist only length and editing metadata
                    // until a future trusted DOM/UIA policy explicitly marks
                    // the field as safe.
                    tp.text = "[UNOBSERVED_TEXT]".to_string();
                }
                tp.is_redacted = true;
            }
            ActionParameters::Clipboard(cp) => {
                if let Some(ref preview) = cp.redacted_preview {
                    let (clean, _) = self.redact_text(preview);
                    cp.redacted_preview = Some(clean);
                }
            }
            _ => {}
        }
    }

    /// Redact a raw event if it targets password elements or sensitive fields.
    pub fn redact_raw_event(&self, event: &mut RawEvent) {
        match &mut event.payload {
            RawEventPayload::Keyboard(kb) if is_printable_virtual_key(kb.vk_code) => {
                // Raw event storage must not become a keylogger. Canonical
                // correlation consumes the original event in memory before
                // this persistence-bound redaction step.
                kb.vk_code = 0;
                kb.scan_code = 0;
                kb.key_name = "[UNOBSERVED_TEXT]".to_string();
            }
            _ => {}
        }
    }
}

fn is_printable_virtual_key(vk_code: u32) -> bool {
    matches!(vk_code, 0x20 | 0x30..=0x5A | 0x60..=0x6F | 0xBA..=0xDF)
}
