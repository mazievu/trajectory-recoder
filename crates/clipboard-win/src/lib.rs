//! Clipboard listener, format metadata extractor, and SHA-256 digest generator.
//! Redacts and avoids persisting raw clipboard data by default.

pub mod formats;
pub mod hasher;
pub mod listener;
pub mod manager;

pub use formats::format_id_to_name;
pub use hasher::{compute_sha256, compute_sha256_str};
pub use manager::ClipboardManager;

#[cfg(test)]
mod tests {
    use super::*;
    use core_types::event::RawEventPayload;
    use std::time::Duration;

    #[test]
    fn test_sha256_hashing() {
        let hash = compute_sha256_str("Hello Trajectory");
        assert_eq!(
            hash,
            "1726029411759a120139aa1988cb1649c3a9bf0c9a58df9134e8bf3f616c1306"
        );

        let empty_hash = compute_sha256(b"");
        assert_eq!(
            empty_hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_format_id_to_name() {
        assert_eq!(format_id_to_name(1), "CF_TEXT");
        assert_eq!(format_id_to_name(13), "CF_UNICODETEXT");
        assert_eq!(format_id_to_name(15), "CF_HDROP");
        assert_eq!(format_id_to_name(2), "CF_BITMAP");
    }

    #[test]
    fn test_clipboard_manager_simulation_pipeline() {
        let mgr = ClipboardManager::start_mock("test_pc", 1, "test_user");
        let rx = mgr.receiver();

        mgr.simulate_copy("CF_UNICODETEXT", b"Secret Password Text", Some(0x5678));

        let event = rx.recv_timeout(Duration::from_millis(100)).unwrap();
        if let RawEventPayload::Clipboard(c) = event.payload {
            assert_eq!(c.format, "CF_UNICODETEXT");
            assert_eq!(c.byte_length, 20);
            assert_eq!(c.source_hwnd, Some(0x5678));
            // Ensure hash is 64 hex chars and raw plaintext is NOT in struct
            assert_eq!(c.hash_sha256.len(), 64);
            assert_eq!(c.hash_sha256, compute_sha256(b"Secret Password Text"));
        } else {
            panic!("Expected Clipboard payload");
        }
    }
}
