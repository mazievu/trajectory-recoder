use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

static BASE_INSTANT: OnceLock<Instant> = OnceLock::new();

fn get_base_instant() -> &'static Instant {
    BASE_INSTANT.get_or_init(Instant::now)
}

/// Dual-timestamp clock combining synchronized UTC wall time and high-resolution monotonic nanoseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DualTimestamp {
    /// UTC wall-clock time in RFC 3339 format for human search and database indexing.
    pub wall_time_utc: DateTime<Utc>,
    /// High-resolution monotonic nanoseconds (QPC or Instant) for sub-millisecond correlation.
    pub monotonic_ns: u64,
    /// Local machine timezone offset from UTC in seconds at event capture time.
    pub timezone_offset_secs: i32,
}

pub type EventTimestamp = DualTimestamp;

impl DualTimestamp {
    /// Creates a DualTimestamp from explicit components.
    #[inline]
    pub const fn from_parts(
        wall_time_utc: DateTime<Utc>,
        monotonic_ns: u64,
        timezone_offset_secs: i32,
    ) -> Self {
        Self {
            wall_time_utc,
            monotonic_ns,
            timezone_offset_secs,
        }
    }

    /// Captures the current dual timestamp from system clock and process monotonic clock.
    pub fn now() -> Self {
        let wall_time_utc = Utc::now();
        let base = get_base_instant();
        let monotonic_ns = Instant::now().duration_since(*base).as_nanos() as u64;
        Self {
            wall_time_utc,
            monotonic_ns,
            timezone_offset_secs: 0,
        }
    }

    /// Computes the elapsed monotonic duration between this timestamp and an earlier one.
    #[inline]
    pub fn duration_since(&self, earlier: &Self) -> Option<Duration> {
        if self.monotonic_ns >= earlier.monotonic_ns {
            Some(Duration::from_nanos(self.monotonic_ns - earlier.monotonic_ns))
        } else {
            None
        }
    }

    /// Computes the elapsed monotonic milliseconds between this timestamp and an earlier one.
    #[inline]
    pub fn elapsed_ms_since(&self, earlier: &Self) -> Option<u64> {
        self.duration_since(earlier).map(|d| d.as_millis() as u64)
    }

    /// Checks if two events occurred within a specified monotonic time delta.
    #[inline]
    pub fn is_within_window(&self, other: &Self, max_delta: Duration) -> bool {
        let delta_ns = self.monotonic_ns.abs_diff(other.monotonic_ns);
        delta_ns <= max_delta.as_nanos() as u64
    }
}

impl fmt::Display for DualTimestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} (mono: {} ns, tz: {}s)",
            self.wall_time_utc.to_rfc3339(),
            self.monotonic_ns,
            self.timezone_offset_secs
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dual_timestamp_monotonic_ordering() {
        let t1 = DualTimestamp::from_parts(Utc::now(), 100_000_000, 0);
        let t2 = DualTimestamp::from_parts(Utc::now(), 200_000_000, 0);
        assert!(t2.monotonic_ns > t1.monotonic_ns);
        assert_eq!(t2.elapsed_ms_since(&t1), Some(100));
        assert!(t2.is_within_window(&t1, Duration::from_millis(150)));
        assert!(!t2.is_within_window(&t1, Duration::from_millis(50)));
    }

    #[test]
    fn test_dual_timestamp_serde_roundtrip() {
        let ts = DualTimestamp::from_parts(Utc::now(), 987654321, 25200);
        let serialized = serde_json::to_string(&ts).unwrap();
        let deserialized: DualTimestamp = serde_json::from_str(&serialized).unwrap();
        assert_eq!(ts, deserialized);
    }
}
