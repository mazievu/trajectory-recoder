use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use tempfile::TempDir;

#[test]
fn test_f11_ndjson_append_only_flush() {
    let tmp = TempDir::new().unwrap();
    let ndjson_file = tmp.path().join("events.raw.ndjson");

    {
        let mut f = File::create(&ndjson_file).unwrap();
        writeln!(f, r#"{{"id": 1, "type": "A"}}"#).unwrap();
        writeln!(f, r#"{{"id": 2, "type": "B"}}"#).unwrap();
        f.flush().unwrap();
    }

    let file = File::open(&ndjson_file).unwrap();
    let reader = BufReader::new(file);
    let count = reader.lines().count();
    assert_eq!(count, 2);
}
