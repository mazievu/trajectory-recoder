use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoTimeRange {
    pub monitor_id: u32,
    pub start_monotonic_ns: u64,
    pub end_monotonic_ns: u64,
}

#[test]
fn test_f20_video_fragment_time_range_validity() {
    let range = VideoTimeRange {
        monitor_id: 0,
        start_monotonic_ns: 10_000_000_000,
        end_monotonic_ns: 12_000_000_000, // 2 seconds
    };

    assert!(range.end_monotonic_ns > range.start_monotonic_ns);
    let duration_ns = range.end_monotonic_ns - range.start_monotonic_ns;
    assert_eq!(duration_ns, 2_000_000_000);
}
