use std::io::Write;
use tempfile::TempDir;

#[test]
fn test_f23_zstd_compression_roundtrip() {
    let original =
        b"Sample trajectory manifest and ndjson lines to be compressed by Zstandard engine.";

    // Compress
    let mut encoder = zstd::Encoder::new(Vec::new(), 3).unwrap();
    encoder.write_all(original).unwrap();
    let compressed = encoder.finish().unwrap();

    // Decompress
    let mut decoder = zstd::Decoder::new(&compressed[..]).unwrap();
    let mut decompressed = Vec::new();
    std::io::copy(&mut decoder, &mut decompressed).unwrap();

    assert_eq!(&decompressed[..], original);
}
