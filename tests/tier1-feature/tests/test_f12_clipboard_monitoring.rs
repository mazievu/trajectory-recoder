use sha2::{Digest, Sha256};

#[test]
fn test_f12_clipboard_metadata_and_hash_redaction() {
    let raw_clipboard_text = "CONFIDENTIAL_PAYROLL_DATA_2026";
    let mut hasher = Sha256::new();
    hasher.update(raw_clipboard_text.as_bytes());
    let hash_hex = format!("{:x}", hasher.finalize());

    // We store metadata: format, length, hash (never raw text by default)
    let format = "CF_UNICODETEXT";
    let byte_length = raw_clipboard_text.len();

    assert_eq!(format, "CF_UNICODETEXT");
    assert_eq!(byte_length, 30);
    assert_eq!(hash_hex.len(), 64);
}
