use core_types::action::CanonicalAction;
use core_types::id::SessionId;
use core_types::timestamp::DualTimestamp;
use rusqlite::{Connection, Result, params};
use std::path::{Path, PathBuf};

/// SQLite WAL persistence database for session indexes and canonical action metadata.
pub struct SessionDatabase {
    path: PathBuf,
    conn: Connection,
}

impl SessionDatabase {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let p = path.as_ref().to_path_buf();
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let conn = Connection::open(&p)?;

        // Enable WAL mode & performance pragmas
        conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA temp_store = MEMORY;
            PRAGMA cache_size = -64000;
            ",
        )?;

        let db = Self { path: p, conn };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS session_meta (
                session_id TEXT PRIMARY KEY,
                machine_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                start_time_utc TEXT NOT NULL,
                start_monotonic_ns INTEGER NOT NULL,
                end_time_utc TEXT,
                end_monotonic_ns INTEGER,
                status TEXT NOT NULL,
                total_events INTEGER DEFAULT 0,
                total_actions INTEGER DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS raw_events (
                global_event_id INTEGER PRIMARY KEY,
                session_event_id INTEGER NOT NULL,
                timestamp_utc TEXT NOT NULL,
                timestamp_monotonic_ns INTEGER NOT NULL,
                source TEXT NOT NULL,
                event_type TEXT NOT NULL,
                payload_json TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS canonical_actions (
                global_event_id INTEGER PRIMARY KEY,
                session_event_id INTEGER NOT NULL,
                timestamp_utc TEXT NOT NULL,
                timestamp_monotonic_ns INTEGER NOT NULL,
                action_type TEXT NOT NULL,
                confidence REAL NOT NULL,
                target_json TEXT,
                context_json TEXT,
                parameters_json TEXT,
                duration_ms INTEGER
            );

            CREATE TABLE IF NOT EXISTS screenshots (
                screenshot_id INTEGER PRIMARY KEY AUTOINCREMENT,
                global_event_id INTEGER,
                timestamp_monotonic_ns INTEGER NOT NULL,
                monitor_id INTEGER NOT NULL,
                file_path TEXT NOT NULL,
                width INTEGER NOT NULL,
                height INTEGER NOT NULL,
                format TEXT NOT NULL,
                byte_size INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS video_segments (
                segment_id INTEGER PRIMARY KEY,
                file_name TEXT NOT NULL,
                start_monotonic_ns INTEGER NOT NULL,
                end_monotonic_ns INTEGER NOT NULL,
                frame_count INTEGER NOT NULL,
                fps INTEGER NOT NULL,
                bitrate_kbps INTEGER NOT NULL,
                width INTEGER NOT NULL,
                height INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS annotations (
                annotation_id INTEGER PRIMARY KEY AUTOINCREMENT,
                global_event_id INTEGER NOT NULL,
                note TEXT NOT NULL,
                tag TEXT,
                created_at_utc TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS id_allocator (
                key TEXT PRIMARY KEY,
                last_allocated_id INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_actions_type ON canonical_actions(action_type);
            CREATE INDEX IF NOT EXISTS idx_actions_ts ON canonical_actions(timestamp_monotonic_ns);
            CREATE INDEX IF NOT EXISTS idx_events_ts ON raw_events(timestamp_monotonic_ns);
            ",
        )?;
        Ok(())
    }

    pub fn insert_session_meta(
        &self,
        session_id: &SessionId,
        machine_id: &str,
        user_id: &str,
        start_time: &DualTimestamp,
        status: &str,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO session_meta (
                session_id, machine_id, user_id, start_time_utc, start_monotonic_ns, status
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                session_id.as_str(),
                machine_id,
                user_id,
                start_time.wall_time_utc.to_rfc3339(),
                start_time.monotonic_ns as i64,
                status,
            ],
        )?;
        Ok(())
    }

    pub fn finalize_session_meta(
        &self,
        session_id: &SessionId,
        end_time: &DualTimestamp,
        total_events: usize,
        total_actions: usize,
        status: &str,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE session_meta SET
                end_time_utc = ?1,
                end_monotonic_ns = ?2,
                total_events = ?3,
                total_actions = ?4,
                status = ?5
            WHERE session_id = ?6",
            params![
                end_time.wall_time_utc.to_rfc3339(),
                end_time.monotonic_ns as i64,
                total_events as i64,
                total_actions as i64,
                status,
                session_id.as_str(),
            ],
        )?;
        Ok(())
    }

    pub fn insert_canonical_action(&self, action: &CanonicalAction) -> Result<()> {
        let target_json = serde_json::to_string(&action.target).unwrap_or_default();
        let context_json = serde_json::to_string(&action.context).unwrap_or_default();
        let params_json = serde_json::to_string(&action.parameters).unwrap_or_default();
        let action_type_str = format!("{:?}", action.action_type);

        self.conn.execute(
            "INSERT OR REPLACE INTO canonical_actions (
                global_event_id, session_event_id, timestamp_utc, timestamp_monotonic_ns,
                action_type, confidence, target_json, context_json, parameters_json, duration_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                action.global_event_id.as_u64() as i64,
                action.session_event_id as i64,
                action.timestamp.wall_time_utc.to_rfc3339(),
                action.timestamp.monotonic_ns as i64,
                action_type_str,
                action.confidence,
                target_json,
                context_json,
                params_json,
                action.duration_ms.map(|d| d as i64),
            ],
        )?;
        Ok(())
    }

    pub fn checkpoint_wal(&self) -> Result<()> {
        self.conn
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_types::action::{ActionParameters, ActionType, CanonicalActionBuilder};
    use core_types::id::GlobalEventId;
    use tempfile::tempdir;

    #[test]
    fn test_sqlite_wal_schema_and_insert() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("session.db");

        let db = SessionDatabase::open(&db_path).unwrap();
        let sid = SessionId::new("test_sess_01");
        let ts = DualTimestamp::now();

        db.insert_session_meta(&sid, "M1", "U1", &ts, "RECORDING")
            .unwrap();

        let action = CanonicalActionBuilder::new(
            GlobalEventId::new(10),
            sid.clone(),
            1,
            ts,
            ActionType::Click,
            ActionParameters::None,
        )
        .build();

        db.insert_canonical_action(&action).unwrap();
        db.finalize_session_meta(&sid, &ts, 1, 1, "FINALIZED")
            .unwrap();
        db.checkpoint_wal().unwrap();
    }
}
