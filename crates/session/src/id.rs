use chrono::{DateTime, Utc};
use core_types::id::SessionId;
use std::sync::atomic::{AtomicU32, Ordering};

static CRASH_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Generates standardized monotonic session IDs: `{machine_id}_{YYYYMMDD}_{HH0000}_{uuid_short}`
pub struct SessionIdGenerator {
    machine_id: String,
}

impl SessionIdGenerator {
    pub fn new(machine_id: impl Into<String>) -> Self {
        Self {
            machine_id: machine_id.into(),
        }
    }

    /// Generate a session ID for the given timestamp.
    pub fn generate(&self, time: DateTime<Utc>) -> SessionId {
        let date_str = time.format("%Y%m%d").to_string();
        let hour_str = time.format("%H0000").to_string();
        let rand_suffix = &uuid::Uuid::new_v4().to_string()[..8];

        let id_str = format!(
            "{}_{}_{}_{}",
            self.machine_id, date_str, hour_str, rand_suffix
        );
        SessionId::new(id_str)
    }

    /// Increment crash counter and generate a recovery session ID.
    pub fn generate_recovered(&self, original_id: &str) -> SessionId {
        let counter = CRASH_COUNTER.fetch_add(1, Ordering::SeqCst) + 1;
        SessionId::new(format!("{}_rec{}", original_id, counter))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_id_format() {
        let generator = SessionIdGenerator::new("WS01");
        let now = Utc::now();
        let sid = generator.generate(now);

        assert!(sid.as_str().starts_with("WS01_"));
        assert_eq!(sid.as_str().split('_').count(), 4);
    }
}
