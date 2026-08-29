use crate::db::SessionDatabase;
use crate::id::SessionIdGenerator;
use crate::ndjson::NdjsonWriter;
use crate::rotation::HourlyRotationTrigger;
use chrono::{DateTime, Utc};
use core_types::action::CanonicalAction;
use core_types::event::RawEvent;
use core_types::id::SessionId;
use core_types::timestamp::DualTimestamp;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::error;

pub const REQUIRED_SUBDIRECTORIES: [&str; 5] = [
    "screenshots",
    "video",
    "browser",
    "uia",
    "diagnostics",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionManifest {
    pub schema: String,
    pub schema_version: String,
    pub session_id: String,
    pub machine_id: String,
    pub user_id: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub status: String,
    pub event_count: usize,
    pub action_count: usize,
}

/// High-level session manager handling live writes, hourly partition rotation, and finalization.
pub struct SessionManager {
    spool_root: PathBuf,
    machine_id: String,
    user_id: String,
    id_gen: SessionIdGenerator,
    rotation_trigger: HourlyRotationTrigger,
    current_session_id: SessionId,
    active_dir: PathBuf,
    raw_ndjson_writer: Option<NdjsonWriter>,
    normalized_ndjson_writer: Option<NdjsonWriter>,
    db: Option<SessionDatabase>,
    started_at: DualTimestamp,
    event_count: usize,
    action_count: usize,
}

impl SessionManager {
    pub fn start(
        spool_root: impl AsRef<Path>,
        machine_id: impl Into<String>,
        user_id: impl Into<String>,
    ) -> std::io::Result<Self> {
        let spool_root = spool_root.as_ref().to_path_buf();
        let m_id = machine_id.into();
        let u_id = user_id.into();

        let id_gen = SessionIdGenerator::new(&m_id);
        let now_utc = Utc::now();
        let rotation_trigger = HourlyRotationTrigger::new(now_utc);
        let session_id = id_gen.generate(now_utc);

        let active_dir = spool_root.join("recording").join(session_id.as_str());
        Self::init_session_directory(&active_dir)?;

        let raw_ndjson_path = active_dir.join("events.raw.ndjson");
        let raw_ndjson_writer = NdjsonWriter::open(raw_ndjson_path)?;

        let norm_ndjson_path = active_dir.join("events.normalized.ndjson");
        let normalized_ndjson_writer = NdjsonWriter::open(norm_ndjson_path)?;

        let db_path = active_dir.join("session.db");
        let db = SessionDatabase::open(db_path).map_err(|e| {
            error!("Failed to open session.db at start: {}", e);
            std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
        })?;

        let start_ts = DualTimestamp::now();
        db.insert_session_meta(&session_id, &m_id, &u_id, &start_ts, "RECORDING").map_err(|e| {
            error!("Failed to insert session_meta at start: {}", e);
            std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
        })?;

        let manifest = SessionManifest {
            schema: "gtf.trajectory".to_string(),
            schema_version: "1.0".to_string(),
            session_id: session_id.as_str().to_string(),
            machine_id: m_id.clone(),
            user_id: u_id.clone(),
            started_at: start_ts.wall_time_utc.to_rfc3339(),
            ended_at: None,
            status: "RECORDING".to_string(),
            event_count: 0,
            action_count: 0,
        };
        Self::write_manifest(&active_dir, &manifest)?;

        Ok(Self {
            spool_root,
            machine_id: m_id,
            user_id: u_id,
            id_gen,
            rotation_trigger,
            current_session_id: session_id,
            active_dir,
            raw_ndjson_writer: Some(raw_ndjson_writer),
            normalized_ndjson_writer: Some(normalized_ndjson_writer),
            db: Some(db),
            started_at: start_ts,
            event_count: 0,
            action_count: 0,
        })
    }

    fn init_session_directory(dir: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dir)?;
        for sub in &REQUIRED_SUBDIRECTORIES {
            std::fs::create_dir_all(dir.join(sub))?;
        }
        Ok(())
    }

    fn write_manifest(dir: &Path, manifest: &SessionManifest) -> std::io::Result<()> {
        let manifest_path = dir.join("manifest.json");
        let json_str = serde_json::to_string_pretty(manifest).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e)
        })?;
        std::fs::write(manifest_path, json_str)
    }

    pub fn current_session_id(&self) -> &SessionId {
        &self.current_session_id
    }

    pub fn write_raw_event(&mut self, event: &RawEvent) -> std::io::Result<()> {
        if let Some(ref mut writer) = self.raw_ndjson_writer {
            writer.write_event(event)?;
            self.event_count += 1;
        }
        Ok(())
    }

    pub fn write_canonical_action(&mut self, action: &CanonicalAction) -> std::io::Result<()> {
        if let Some(ref mut writer) = self.normalized_ndjson_writer {
            writer.write_record(action)?;
        }
        if let Some(ref db) = self.db {
            db.insert_canonical_action(action).map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
            })?;
        }
        self.action_count += 1;
        Ok(())
    }

    /// Check for clock-hour rotation and perform atomic session rollover.
    pub fn check_rotation(&mut self) -> std::io::Result<Option<SessionId>> {
        let now = Utc::now();
        if self.rotation_trigger.should_rotate(now) {
            let old_id = self.current_session_id.clone();
            self.rotate_session(now)?;
            Ok(Some(old_id))
        } else {
            Ok(None)
        }
    }

    /// Rotate active session to a new hourly partition and move old session to `finalizing/`.
    pub fn rotate_session(&mut self, now: DateTime<Utc>) -> std::io::Result<()> {
        let end_ts = DualTimestamp::now();

        // 1. Finalize old session meta in SQLite
        if let Some(ref db) = self.db {
            db.finalize_session_meta(
                &self.current_session_id,
                &end_ts,
                self.event_count,
                self.action_count,
                "FINALIZED",
            ).map_err(|e| {
                error!("Failed to finalize session meta on rotation: {}", e);
                std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
            })?;

            // 2. Checkpoint WAL
            db.checkpoint_wal().map_err(|e| {
                error!("Failed to checkpoint SQLite WAL on rotation: {}", e);
                std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
            })?;
        }

        // Flush NDJSON writers
        if let Some(ref mut w) = self.raw_ndjson_writer {
            w.flush_sync().map_err(|e| {
                error!("Failed to flush raw NDJSON on rotation: {}", e);
                e
            })?;
        }
        if let Some(ref mut w) = self.normalized_ndjson_writer {
            w.flush_sync().map_err(|e| {
                error!("Failed to flush normalized NDJSON on rotation: {}", e);
                e
            })?;
        }

        // 3. Update manifest.json with FINALIZED metadata
        let finalized_manifest = SessionManifest {
            schema: "gtf.trajectory".to_string(),
            schema_version: "1.0".to_string(),
            session_id: self.current_session_id.as_str().to_string(),
            machine_id: self.machine_id.clone(),
            user_id: self.user_id.clone(),
            started_at: self.started_at.wall_time_utc.to_rfc3339(),
            ended_at: Some(end_ts.wall_time_utc.to_rfc3339()),
            status: "FINALIZED".to_string(),
            event_count: self.event_count,
            action_count: self.action_count,
        };
        Self::write_manifest(&self.active_dir, &finalized_manifest).map_err(|e| {
            error!("Failed to write finalized manifest.json on rotation: {}", e);
            e
        })?;

        // Explicitly drop open file handles and SQLite database connection to unlock directory on Windows
        drop(self.db.take());
        drop(self.raw_ndjson_writer.take());
        drop(self.normalized_ndjson_writer.take());

        // 4. Move old session dir from recording/ to finalizing/
        let finalizing_dir = self.spool_root.join("finalizing").join(self.current_session_id.as_str());
        if let Some(parent) = finalizing_dir.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::rename(&self.active_dir, &finalizing_dir).map_err(|e| {
            error!("Failed to rename session dir on rotation: {}", e);
            e
        })?;

        // 5. Initialize new session
        let new_session_id = self.id_gen.generate(now);
        let new_active_dir = self.spool_root.join("recording").join(new_session_id.as_str());
        Self::init_session_directory(&new_active_dir)?;

        let new_raw_ndjson = NdjsonWriter::open(new_active_dir.join("events.raw.ndjson"))?;
        let new_norm_ndjson = NdjsonWriter::open(new_active_dir.join("events.normalized.ndjson"))?;

        let new_db_path = new_active_dir.join("session.db");
        let new_db = SessionDatabase::open(new_db_path).map_err(|e| {
            error!("Failed to open new session.db on rotation: {}", e);
            std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
        })?;

        let new_start_ts = DualTimestamp::now();
        new_db.insert_session_meta(&new_session_id, &self.machine_id, &self.user_id, &new_start_ts, "RECORDING").map_err(|e| {
            error!("Failed to insert new session_meta on rotation: {}", e);
            std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
        })?;

        let new_manifest = SessionManifest {
            schema: "gtf.trajectory".to_string(),
            schema_version: "1.0".to_string(),
            session_id: new_session_id.as_str().to_string(),
            machine_id: self.machine_id.clone(),
            user_id: self.user_id.clone(),
            started_at: new_start_ts.wall_time_utc.to_rfc3339(),
            ended_at: None,
            status: "RECORDING".to_string(),
            event_count: 0,
            action_count: 0,
        };
        Self::write_manifest(&new_active_dir, &new_manifest)?;

        self.current_session_id = new_session_id;
        self.active_dir = new_active_dir;
        self.raw_ndjson_writer = Some(new_raw_ndjson);
        self.normalized_ndjson_writer = Some(new_norm_ndjson);
        self.db = Some(new_db);
        self.started_at = new_start_ts;
        self.event_count = 0;
        self.action_count = 0;

        Ok(())
    }
}
