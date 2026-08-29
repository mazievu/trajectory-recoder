use e2e_runner::verifiers::*;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

#[test]
fn test_ndjson_verifier_valid_and_monotonic() {
    let tmp = TempDir::new().unwrap();
    let ndjson_path = tmp.path().join("events.raw.ndjson");

    let mut file = File::create(&ndjson_path).unwrap();
    writeln!(file, r#"{{"global_event_id": 1001, "event_type": "CLICK", "timestamp": {{"wall_time_utc": "2026-08-29T03:00:00Z", "monotonic_ns": 1000000, "timezone_offset_secs": 0}}}}"#).unwrap();
    writeln!(file, r#"{{"global_event_id": 1002, "event_type": "TYPE", "timestamp": {{"wall_time_utc": "2026-08-29T03:00:01Z", "monotonic_ns": 2000000, "timezone_offset_secs": 0}}, "text": "[REDACTED]"}}"#).unwrap();
    writeln!(file, r#"{{"global_event_id": 1003, "event_type": "SCROLL", "timestamp": {{"wall_time_utc": "2026-08-29T03:00:02Z", "monotonic_ns": 3000000, "timezone_offset_secs": 0}}}}"#).unwrap();
    file.flush().unwrap();

    let report =
        NdjsonVerifier::verify_raw_ndjson(&ndjson_path, &["SecretPassword", "1234-5678"]).unwrap();
    assert_eq!(report.total_lines, 3);
    assert_eq!(report.valid_lines, 3);
    assert_eq!(report.corrupted_lines, 0);
    assert_eq!(report.min_global_event_id, Some(1001));
    assert_eq!(report.max_global_event_id, Some(1003));
    assert!(report.is_strictly_monotonic);
    assert!(report.sensitive_leaks_detected.is_empty());
}

#[test]
fn test_ndjson_verifier_detects_plaintext_leak() {
    let tmp = TempDir::new().unwrap();
    let ndjson_path = tmp.path().join("events.raw.ndjson");

    let mut file = File::create(&ndjson_path).unwrap();
    writeln!(
        file,
        r#"{{"global_event_id": 2001, "event_type": "TYPE", "text": "SecretPassword123"}}"#
    )
    .unwrap();
    file.flush().unwrap();

    let report = NdjsonVerifier::verify_raw_ndjson(&ndjson_path, &["SecretPassword123"]).unwrap();
    assert_eq!(report.sensitive_leaks_detected.len(), 1);
}

#[test]
fn test_crypto_verifier_roundtrip_and_tamper_rejection() {
    let key = CryptoVerifier::generate_key();
    let nonce = CryptoVerifier::generate_nonce();
    let original = b"Highly sensitive captured desktop trajectory payload bytes";

    let encrypted = CryptoVerifier::encrypt_payload(&key, &nonce, original).unwrap();
    assert_ne!(encrypted, original);

    let decrypted = CryptoVerifier::decrypt_payload(&key, &nonce, &encrypted).unwrap();
    assert_eq!(decrypted, original);

    let tamper_check = CryptoVerifier::verify_tamper_rejection(&key, &nonce, original);
    assert!(tamper_check.is_ok());
}

#[test]
fn test_screenshot_verifier_webp_magic() {
    let mut valid_webp = vec![0u8; 32];
    valid_webp[0..4].copy_from_slice(b"RIFF");
    valid_webp[8..12].copy_from_slice(b"WEBP");

    assert!(ScreenshotVerifier::is_valid_webp_header(&valid_webp));

    let invalid = vec![0u8; 32];
    assert!(!ScreenshotVerifier::is_valid_webp_header(&invalid));
}
