use std::collections::HashMap;

/// Calculate Shannon entropy in bits per character for a given string slice.
/// $H(X) = -\sum_{x} p(x) \log_2 p(x)$
pub fn calculate_shannon_entropy(s: &str) -> f64 {
    if s.is_empty() {
        return 0.0;
    }

    let mut freq = HashMap::new();
    let mut total_chars = 0usize;

    for ch in s.chars() {
        *freq.entry(ch).or_insert(0usize) += 1;
        total_chars += 1;
    }

    let len_f = total_chars as f64;
    let mut entropy = 0.0;

    for &count in freq.values() {
        let p = (count as f64) / len_f;
        if p > 0.0 {
            entropy -= p * p.log2();
        }
    }

    entropy
}

/// Check if a string is considered a high-entropy secret ($H > 4.5$ with length $\ge 16$).
pub fn is_high_entropy_secret(s: &str, threshold: f64, min_len: usize) -> bool {
    if s.len() < min_len {
        return false;
    }
    // Only test words that don't contain spaces (potential tokens/keys)
    if s.contains(' ') {
        return false;
    }
    let entropy = calculate_shannon_entropy(s);
    entropy > threshold
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shannon_entropy_calculation() {
        // Uniform string
        assert_eq!(calculate_shannon_entropy("AAAAAA"), 0.0);

        // Standard English word: low entropy
        let word_entropy = calculate_shannon_entropy("hello world");
        assert!(word_entropy < 3.5);

        // High entropy random string (e.g. base64 / token)
        let token = "7xK9$mP2!qR5vL8#wZ1*yN4&jB6^hT3@";
        let token_entropy = calculate_shannon_entropy(token);
        assert!(token_entropy > 4.5);
        assert!(is_high_entropy_secret(token, 4.5, 16));
    }
}
