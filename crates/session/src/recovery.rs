use std::fs::{self, OpenOptions};
use std::io::Read;
use std::path::{Path, PathBuf};

/// Result of a crash recovery scan on an orphaned session directory.
#[derive(Debug, Clone, PartialEq)]
pub struct RecoveryResult {
    pub session_id: String,
    pub path: PathBuf,
    pub recovered_events: usize,
    pub bytes_truncated: usize,
    pub status: &'static str, // "RECOVERED", "CLEAN", "CORRUPT"
}

/// Scan `spool/recording/` and repair partial writes or corrupt trailing lines.
pub fn scan_and_recover_orphaned_sessions(recording_dir: impl AsRef<Path>) -> Vec<RecoveryResult> {
    let mut results = Vec::new();
    let dir = recording_dir.as_ref();
    if !dir.exists() {
        return results;
    }

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let sid = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                let ndjson_path = path.join("events.raw.ndjson");
                let db_path = path.join("session.db");

                let (events, truncated) = repair_ndjson_tail(&ndjson_path);

                // Update session.db status to RECOVERED if db exists
                if db_path.exists() {
                    if let Ok(conn) = rusqlite::Connection::open(&db_path) {
                        let _ = conn.execute(
                            "UPDATE session_meta SET status = 'RECOVERED' WHERE session_id = ?1",
                            rusqlite::params![sid],
                        );
                        let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
                    }
                }

                results.push(RecoveryResult {
                    session_id: sid,
                    path: path.clone(),
                    recovered_events: events,
                    bytes_truncated: truncated,
                    status: "RECOVERED",
                });
            }
        }
    }

    results
}

/// Scan `events.raw.ndjson`, validate line boundaries, and truncate partial corrupt tail.
pub fn repair_ndjson_tail(path: impl AsRef<Path>) -> (usize, usize) {
    let p = path.as_ref();
    if !p.exists() {
        return (0, 0);
    }

    let Ok(mut file) = OpenOptions::new().read(true).write(true).open(p) else {
        return (0, 0);
    };

    let mut buf = Vec::new();
    if file.read_to_end(&mut buf).is_err() {
        return (0, 0);
    }

    let mut valid_events = 0usize;
    let mut last_valid_pos = 0usize;
    let mut current_pos = 0usize;

    for line in buf.split(|&b| b == b'\n') {
        let line_len = line.len();
        if line_len == 0 {
            current_pos += 1; // newline char
            continue;
        }

        // Check if valid JSON
        if serde_json::from_slice::<serde_json::Value>(line).is_ok() {
            valid_events += 1;
            current_pos += line_len + 1;
            last_valid_pos = current_pos;
        } else {
            // Found corrupt/partial line at tail
            break;
        }
    }

    let bytes_truncated = buf.len().saturating_sub(last_valid_pos);
    if bytes_truncated > 0 {
        let _ = file.set_len(last_valid_pos as u64);
        let _ = file.sync_all();
    }

    (valid_events, bytes_truncated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_repair_ndjson_tail_with_partial_write() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("events.raw.ndjson");

        // Write 2 valid lines and 1 half-written corrupted tail line
        let content = b"{\"event\": 1}\n{\"event\": 2}\n{\"event\": 3, \"corrupt_partial_line...";
        fs::write(&path, content).unwrap();

        let (valid, truncated) = repair_ndjson_tail(&path);
        assert_eq!(valid, 2);
        assert!(truncated > 0);

        let repaired_content = fs::read_to_string(&path).unwrap();
        assert_eq!(repaired_content, "{\"event\": 1}\n{\"event\": 2}\n");
    }
}
