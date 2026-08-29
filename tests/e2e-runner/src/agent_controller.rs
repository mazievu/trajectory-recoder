use rusqlite::Connection;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

pub struct SpoolDirectoryManager {
    pub base_path: PathBuf,
}

impl SpoolDirectoryManager {
    pub fn new(base_path: impl Into<PathBuf>) -> Result<Self, std::io::Error> {
        let base = base_path.into();
        for state_dir in &["recording", "finalizing", "pending_upload", "uploading", "uploaded", "failed"] {
            fs::create_dir_all(base.join(state_dir))?;
        }
        Ok(Self { base_path: base })
    }

    pub fn recording_dir(&self) -> PathBuf {
        self.base_path.join("recording")
    }

    pub fn finalizing_dir(&self) -> PathBuf {
        self.base_path.join("finalizing")
    }

    pub fn pending_upload_dir(&self) -> PathBuf {
        self.base_path.join("pending_upload")
    }

    pub fn uploaded_dir(&self) -> PathBuf {
        self.base_path.join("uploaded")
    }

    pub fn failed_dir(&self) -> PathBuf {
        self.base_path.join("failed")
    }

    /// Atomic transition: recording -> finalizing
    pub fn transition_recording_to_finalizing(&self, session_id: &str) -> Result<PathBuf, std::io::Error> {
        let src = self.recording_dir().join(session_id);
        let dst = self.finalizing_dir().join(session_id);
        fs::rename(src, &dst)?;
        Ok(dst)
    }

    /// Atomic transition: finalizing -> pending_upload
    pub fn transition_finalizing_to_pending(&self, session_id: &str) -> Result<PathBuf, std::io::Error> {
        let src = self.finalizing_dir().join(session_id);
        let dst = self.pending_upload_dir().join(session_id);
        fs::rename(src, &dst)?;
        Ok(dst)
    }

    /// Atomic transition: pending_upload -> uploaded
    pub fn transition_pending_to_uploaded(&self, session_id: &str) -> Result<PathBuf, std::io::Error> {
        let src = self.pending_upload_dir().join(session_id);
        let dst = self.uploaded_dir().join(session_id);
        fs::rename(src, &dst)?;
        Ok(dst)
    }

    /// Startup crash recovery: scan recording/ for unfinalized sessions and safely truncate tail bytes
    pub fn recover_orphaned_sessions(&self) -> Result<Vec<String>, String> {
        let mut recovered_sessions = Vec::new();
        let entries = fs::read_dir(self.recording_dir()).map_err(|e| e.to_string())?;

        for entry in entries.flatten() {
            let session_path = entry.path();
            if session_path.is_dir() {
                let session_id = session_path.file_name().unwrap().to_string_lossy().to_string();

                // 1. Truncate incomplete raw NDJSON tail
                let raw_file = session_path.join("events.raw.ndjson");
                if raw_file.exists() {
                    Self::truncate_corrupt_ndjson_tail(&raw_file)?;
                }

                // 2. Rebuild SQLite index if missing or corrupt
                let sqlite_file = session_path.join("index.sqlite");
                if raw_file.exists() {
                    Self::rebuild_sqlite_index_from_ndjson(&raw_file, &sqlite_file)?;
                }

                // 3. Move session into pending_upload
                let dst = self.pending_upload_dir().join(&session_id);
                fs::rename(&session_path, &dst).map_err(|e| e.to_string())?;
                recovered_sessions.push(session_id);
            }
        }

        Ok(recovered_sessions)
    }

    /// Truncate file to the last valid newline character
    pub fn truncate_corrupt_ndjson_tail(file_path: &Path) -> Result<usize, String> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(file_path)
            .map_err(|e| e.to_string())?;

        let file_len = file.metadata().map_err(|e| e.to_string())?.len();
        if file_len == 0 {
            return Ok(0);
        }

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).map_err(|e| e.to_string())?;

        let mut last_valid_offset = None;
        let mut offset = 0;

        for line in buffer.split_inclusive(|&b| b == b'\n') {
            if line.ends_with(b"\n") {
                let trimmed = std::str::from_utf8(line).unwrap_or("").trim();
                if !trimmed.is_empty() {
                    if serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
                        offset += line.len();
                        last_valid_offset = Some(offset);
                    } else {
                        break;
                    }
                } else {
                    offset += line.len();
                }
            }
        }

        let target_len = last_valid_offset.unwrap_or(0) as u64;
        file.set_len(target_len).map_err(|e| e.to_string())?;
        file.seek(SeekFrom::End(0)).map_err(|e| e.to_string())?;
        file.flush().map_err(|e| e.to_string())?;

        Ok((file_len - target_len) as usize)
    }

    /// Rebuild SQLite WAL index from NDJSON
    pub fn rebuild_sqlite_index_from_ndjson(raw_ndjson: &Path, sqlite_path: &Path) -> Result<usize, String> {
        let conn = Connection::open(sqlite_path).map_err(|e| e.to_string())?;
        conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            CREATE TABLE IF NOT EXISTS session_meta (
                session_id TEXT PRIMARY KEY,
                started_at TEXT,
                ended_at TEXT,
                status TEXT
            );
            CREATE TABLE IF NOT EXISTS raw_events (
                global_event_id INTEGER PRIMARY KEY,
                event_type TEXT,
                monotonic_ns INTEGER,
                payload TEXT
            );
            CREATE TABLE IF NOT EXISTS canonical_actions (
                global_event_id INTEGER PRIMARY KEY,
                session_id TEXT,
                action_type TEXT,
                confidence REAL,
                target TEXT
            );
            CREATE TABLE IF NOT EXISTS screenshots (
                id INTEGER PRIMARY KEY,
                global_event_id INTEGER,
                file_path TEXT,
                monitor_id INTEGER
            );
            CREATE TABLE IF NOT EXISTS video_segments (
                id INTEGER PRIMARY KEY,
                file_path TEXT,
                start_ns INTEGER,
                end_ns INTEGER
            );
            CREATE TABLE IF NOT EXISTS annotations (
                id INTEGER PRIMARY KEY,
                global_event_id INTEGER,
                note TEXT
            );
            CREATE TABLE IF NOT EXISTS id_allocator (
                next_event_id INTEGER PRIMARY KEY
            );
            ",
        )
        .map_err(|e| e.to_string())?;

        let file = File::open(raw_ndjson).map_err(|e| e.to_string())?;
        let reader = BufReader::new(file);
        let mut count = 0;

        let mut tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        for line_res in reader.lines() {
            if let Ok(line) = line_res {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
                    if let Some(eid) = val.get("global_event_id").and_then(|v| v.as_u64()) {
                        let etype = val.get("event_type").and_then(|v| v.as_str()).unwrap_or("RAW");
                        let mono = val.get("timestamp").and_then(|t| t.get("monotonic_ns")).and_then(|m| m.as_u64()).unwrap_or(0);
                        tx.execute(
                            "INSERT OR REPLACE INTO raw_events (global_event_id, event_type, monotonic_ns, payload) VALUES (?1, ?2, ?3, ?4);",
                            rusqlite::params![eid as i64, etype, mono as i64, trimmed],
                        ).map_err(|e| e.to_string())?;
                        count += 1;
                    }
                }
            }
        }
        tx.commit().map_err(|e| e.to_string())?;

        Ok(count)
    }
}
