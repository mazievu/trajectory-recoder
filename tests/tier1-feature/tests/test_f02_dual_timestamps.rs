use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DualTimestamp {
    pub wall_time_utc: DateTime<Utc>,
    pub monotonic_ns: u64,
    pub timezone_offset_secs: i32,
}

#[test]
fn test_f02_dual_timestamp_ordering_and_monotonicity() {
    let t1 = DualTimestamp {
        wall_time_utc: Utc::now(),
        monotonic_ns: 1_000_000,
        timezone_offset_secs: 25200, // +07:00
    };
    let t2 = DualTimestamp {
        wall_time_utc: Utc::now(),
        monotonic_ns: 1_050_000,
        timezone_offset_secs: 25200,
    };

    assert!(t2.monotonic_ns > t1.monotonic_ns);
    assert_eq!(t2.monotonic_ns - t1.monotonic_ns, 50_000); // 50 microseconds delta
}

#[test]
fn test_f02_dual_timestamp_serde_roundtrip() {
    let ts = DualTimestamp {
        wall_time_utc: Utc::now(),
        monotonic_ns: 99_888_777_666,
        timezone_offset_secs: 0,
    };

    let json_str = serde_json::to_string(&ts).unwrap();
    let deserialized: DualTimestamp = serde_json::from_str(&json_str).unwrap();

    assert_eq!(ts.monotonic_ns, deserialized.monotonic_ns);
    assert_eq!(ts.timezone_offset_secs, deserialized.timezone_offset_secs);
}
