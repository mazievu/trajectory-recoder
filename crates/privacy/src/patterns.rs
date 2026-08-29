use regex::Regex;
use std::sync::LazyLock;

// SSN regex: 3 digits - 2 digits - 4 digits
pub static SSN_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").expect("Valid SSN regex"));

// Generic potential credit card regex (13 to 19 digits, optionally spaced or hyphenated)
pub static CC_CANDIDATE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(?:\d[ -]*?){13,19}\b").expect("Valid CC regex"));

// API keys and common cloud tokens
pub static API_KEY_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:AKIA[0-9A-Z]{16}|ghp_[0-9a-zA-Z]{36}|Bearer\s+[A-Za-z0-9\-_=]+\.[A-Za-z0-9\-_=]+\.?[A-Za-z0-9\-_=]*|sk_live_[0-9a-zA-Z]{24})\b")
        .expect("Valid API Key regex")
});

// URL password regex
pub static URL_PASSWORD_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"https?://([^:]+):([^@]+)@").expect("Valid URL password regex"));

// Email regex
pub static EMAIL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b").expect("Valid email regex")
});

/// Luhn algorithm for validating Credit Card numbers.
pub fn is_valid_luhn_credit_card(number_str: &str) -> bool {
    let digits: Vec<u32> = number_str.chars().filter_map(|c| c.to_digit(10)).collect();

    if digits.len() < 13 || digits.len() > 19 {
        return false;
    }

    let mut sum = 0;
    let mut double = false;

    for &digit in digits.iter().rev() {
        if double {
            let doubled = digit * 2;
            sum += if doubled > 9 { doubled - 9 } else { doubled };
        } else {
            sum += digit;
        }
        double = !double;
    }

    sum % 10 == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_luhn_algorithm() {
        // Valid Visa test card (4532 0151 1283 0366)
        assert!(is_valid_luhn_credit_card("4532-0151-1283-0366"));
        assert!(is_valid_luhn_credit_card("4532 0151 1283 0366"));
        assert!(is_valid_luhn_credit_card("4532015112830366"));

        // Invalid card
        assert!(!is_valid_luhn_credit_card("4532-0151-1283-0367"));
        assert!(!is_valid_luhn_credit_card("1234567890"));
    }

    #[test]
    fn test_ssn_regex() {
        assert!(SSN_REGEX.is_match("User SSN is 000-12-3456 here"));
        assert!(!SSN_REGEX.is_match("12345-6789"));
    }

    #[test]
    fn test_api_key_regex() {
        assert!(API_KEY_REGEX.is_match("AKIAIOSFODNN7EXAMPLE"));
        assert!(API_KEY_REGEX.is_match("ghp_123456789012345678901234567890123456"));
    }
}
