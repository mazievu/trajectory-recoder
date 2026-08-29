use regex::Regex;

fn is_luhn_valid(num_str: &str) -> bool {
    let digits: Vec<u32> = num_str.chars().filter_map(|c| c.to_digit(10)).collect();
    if digits.len() < 13 || digits.len() > 19 {
        return false;
    }
    let mut sum = 0;
    let mut double = false;
    for &d in digits.iter().rev() {
        let val = if double {
            let doubled = d * 2;
            if doubled > 9 { doubled - 9 } else { doubled }
        } else {
            d
        };
        sum += val;
        double = !double;
    }
    sum % 10 == 0
}

#[test]
fn test_f18_privacy_luhn_credit_card_detection() {
    // Standard test card numbers (Luhn valid)
    assert!(is_luhn_valid("4532015112830366"));
    // Invalid card number
    assert!(!is_luhn_valid("4532015112830367"));
}

#[test]
fn test_f18_jwt_token_regex_redaction() {
    let jwt_regex = Regex::new(r"ey[A-Za-z0-9-_]+\.[A-Za-z0-9-_]+\.[A-Za-z0-9-_]+").unwrap();
    let text_with_jwt = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.do_not_leak";
    let redacted = jwt_regex.replace_all(text_with_jwt, "[REDACTED]");

    assert_eq!(redacted, "Authorization: Bearer [REDACTED]");
}
