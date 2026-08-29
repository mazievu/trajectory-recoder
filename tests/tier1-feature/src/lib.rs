//! Tier 1 Feature Coverage Test Suite
//!
//! Provides comprehensive requirement-driven test verification across all core features
//! defined in the Trajectory Recorder Master Specification (Sections 0–75) and
//! Acceptance Criteria (AC 1–AC 40).

pub mod test_helpers {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub struct DualTimestamp {
        pub wall_time_utc: DateTime<Utc>,
        pub monotonic_ns: u64,
        pub timezone_offset_secs: i32,
    }

    impl DualTimestamp {
        pub fn now_with_mono(monotonic_ns: u64) -> Self {
            Self {
                wall_time_utc: Utc::now(),
                monotonic_ns,
                timezone_offset_secs: 0,
            }
        }
    }
}
