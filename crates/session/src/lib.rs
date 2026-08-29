//! Session partition router, hourly rotation, SQLite WAL, and NDJSON persistence.

pub mod db;
pub mod global_id;
pub mod id;
pub mod manager;
pub mod ndjson;
pub mod recovery;
pub mod rotation;

pub use db::SessionDatabase;
pub use global_id::GlobalEventIdAllocator;
pub use id::SessionIdGenerator;
pub use manager::SessionManager;
pub use ndjson::NdjsonWriter;
pub use recovery::{RecoveryResult, repair_ndjson_tail, scan_and_recover_orphaned_sessions};
pub use rotation::HourlyRotationTrigger;

#[cfg(test)]
mod tests {
    use super::*;
    use core_types::action::{ActionParameters, ActionType, CanonicalActionBuilder};
    use core_types::event::{EventSource, RawEvent, RawEventPayload, RawMouseEvent};
    use core_types::id::GlobalEventId;
    use core_types::timestamp::DualTimestamp;
    use tempfile::tempdir;

    #[test]
    fn test_session_lifecycle_and_rotation() {
        let dir = tempdir().unwrap();
        let spool_root = dir.path().join("spool");

        let mut manager = SessionManager::start(&spool_root, "MACH01", "USER01").unwrap();
        let sid1 = manager.current_session_id().clone();

        let raw_ev = RawEvent::new(
            1,
            GlobalEventId::new(1),
            DualTimestamp::now(),
            "MACH01".to_string(),
            1,
            "USER01".to_string(),
            EventSource::Win32Hook,
            1,
            RawEventPayload::Mouse(RawMouseEvent::default()),
        );

        let action = CanonicalActionBuilder::new(
            GlobalEventId::new(1),
            sid1.clone(),
            1,
            DualTimestamp::now(),
            ActionType::Click,
            ActionParameters::None,
        )
        .build();

        manager.write_raw_event(&raw_ev).unwrap();
        manager.write_canonical_action(&action).unwrap();

        // Perform rotation
        let future_hour = chrono::Utc::now() + chrono::Duration::hours(2);
        manager.rotate_session(future_hour).unwrap();

        let sid2 = manager.current_session_id().clone();
        assert_ne!(sid1, sid2);

        // Verify old session moved to finalizing/
        let old_finalizing = spool_root.join("finalizing").join(sid1.as_str());
        assert!(old_finalizing.exists());
        assert!(old_finalizing.join("events.raw.ndjson").exists());
        assert!(old_finalizing.join("events.normalized.ndjson").exists());
        assert!(old_finalizing.join("session.db").exists());
        assert!(old_finalizing.join("manifest.json").exists());
    }
}
