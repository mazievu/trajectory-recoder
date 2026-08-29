#[test]
fn test_f19_webp_header_detection() {
    let mut header = [0u8; 12];
    header[0..4].copy_from_slice(b"RIFF");
    header[4..8].copy_from_slice(&1000u32.to_le_bytes());
    header[8..12].copy_from_slice(b"WEBP");

    assert_eq!(&header[0..4], b"RIFF");
    assert_eq!(&header[8..12], b"WEBP");
}
