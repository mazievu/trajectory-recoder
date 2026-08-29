use crate::priority::Priority;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

/// Snapshot of event bus performance and drop metrics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct EventBusMetricsSnapshot {
    pub published_p0: u64,
    pub published_p1: u64,
    pub published_p2: u64,
    pub published_p3: u64,
    pub published_p4: u64,

    pub dropped_p0: u64,
    pub dropped_p1: u64,
    pub dropped_p2: u64,
    pub dropped_p3: u64,
    pub dropped_p4: u64,

    pub queue_depth_p0: usize,
    pub queue_depth_p1: usize,
    pub queue_depth_p2: usize,
    pub queue_depth_p3: usize,
    pub queue_depth_p4: usize,

    pub total_published: u64,
    pub total_dropped: u64,
    pub total_consumed: u64,
}

/// Internal atomic counters tracking throughput and shed drops.
pub struct EventBusMetrics {
    published: [AtomicU64; 5],
    dropped: [AtomicU64; 5],
    consumed: AtomicU64,
}

impl Default for EventBusMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBusMetrics {
    pub fn new() -> Self {
        Self {
            published: [
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
            ],
            dropped: [
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
            ],
            consumed: AtomicU64::new(0),
        }
    }

    #[inline]
    pub fn record_published(&self, priority: Priority) {
        self.published[priority as usize].fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn record_dropped(&self, priority: Priority) {
        self.dropped[priority as usize].fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn record_consumed(&self) {
        self.consumed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self, depths: [usize; 5]) -> EventBusMetricsSnapshot {
        let pub_p0 = self.published[0].load(Ordering::Relaxed);
        let pub_p1 = self.published[1].load(Ordering::Relaxed);
        let pub_p2 = self.published[2].load(Ordering::Relaxed);
        let pub_p3 = self.published[3].load(Ordering::Relaxed);
        let pub_p4 = self.published[4].load(Ordering::Relaxed);

        let drop_p0 = self.dropped[0].load(Ordering::Relaxed);
        let drop_p1 = self.dropped[1].load(Ordering::Relaxed);
        let drop_p2 = self.dropped[2].load(Ordering::Relaxed);
        let drop_p3 = self.dropped[3].load(Ordering::Relaxed);
        let drop_p4 = self.dropped[4].load(Ordering::Relaxed);

        let total_pub = pub_p0 + pub_p1 + pub_p2 + pub_p3 + pub_p4;
        let total_drop = drop_p0 + drop_p1 + drop_p2 + drop_p3 + drop_p4;
        let total_consumed = self.consumed.load(Ordering::Relaxed);

        EventBusMetricsSnapshot {
            published_p0: pub_p0,
            published_p1: pub_p1,
            published_p2: pub_p2,
            published_p3: pub_p3,
            published_p4: pub_p4,
            dropped_p0: drop_p0,
            dropped_p1: drop_p1,
            dropped_p2: drop_p2,
            dropped_p3: drop_p3,
            dropped_p4: drop_p4,
            queue_depth_p0: depths[0],
            queue_depth_p1: depths[1],
            queue_depth_p2: depths[2],
            queue_depth_p3: depths[3],
            queue_depth_p4: depths[4],
            total_published: total_pub,
            total_dropped: total_drop,
            total_consumed,
        }
    }
}
