use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// High-throughput in-memory atomic metrics collector.
#[derive(Debug)]
pub struct MetricsCollector {
    // Event counters
    pub events_captured_total: AtomicU64,
    pub events_dropped_total: AtomicU64,
    pub canonical_actions_total: AtomicU64,
    
    // Channel queue depths (P0 to P4)
    pub queue_depth_p0_input: AtomicUsize,
    pub queue_depth_p1_window: AtomicUsize,
    pub queue_depth_p2_dom_uia: AtomicUsize,
    pub queue_depth_p3_screenshot: AtomicUsize,
    pub queue_depth_p4_video: AtomicUsize,

    // Latency trackers (in microseconds)
    pub last_uia_latency_us: AtomicU64,
    pub max_uia_latency_us: AtomicU64,
    pub last_screenshot_latency_us: AtomicU64,
    pub last_disk_write_latency_us: AtomicU64,

    // Video pipeline
    pub video_encoder_fps: AtomicU32,

    // Resource metrics
    pub memory_working_set_bytes: AtomicU64,
    pub cpu_usage_pct_x100: AtomicU32, // e.g. 250 = 2.50%

    // Spool & Upload
    pub pending_upload_bytes: AtomicU64,
    pub upload_throughput_bps: AtomicU64,
    pub failed_chunks_total: AtomicU64,

    // Rate calculation state
    rate_state: RwLock<RateCalculationState>,
}

#[derive(Debug)]
struct RateCalculationState {
    last_sample_instant: Instant,
    last_events_captured: u64,
    last_events_dropped: u64,
    last_bytes_uploaded: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub timestamp_utc: chrono::DateTime<chrono::Utc>,
    pub events_per_sec: f64,
    pub dropped_per_sec: f64,
    pub events_captured_total: u64,
    pub events_dropped_total: u64,
    pub canonical_actions_total: u64,
    pub queue_depths: QueueDepths,
    pub latencies_us: LatencyMetrics,
    pub video_fps: u32,
    pub memory_working_set_bytes: u64,
    pub cpu_usage_pct: f32,
    pub pending_upload_bytes: u64,
    pub upload_throughput_bytes_sec: u64,
    pub failed_chunks_total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueDepths {
    pub p0_input: usize,
    pub p1_window: usize,
    pub p2_dom_uia: usize,
    pub p3_screenshot: usize,
    pub p4_video: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyMetrics {
    pub last_uia_latency_us: u64,
    pub max_uia_latency_us: u64,
    pub last_screenshot_latency_us: u64,
    pub last_disk_write_latency_us: u64,
}

impl MetricsCollector {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            events_captured_total: AtomicU64::new(0),
            events_dropped_total: AtomicU64::new(0),
            canonical_actions_total: AtomicU64::new(0),
            queue_depth_p0_input: AtomicUsize::new(0),
            queue_depth_p1_window: AtomicUsize::new(0),
            queue_depth_p2_dom_uia: AtomicUsize::new(0),
            queue_depth_p3_screenshot: AtomicUsize::new(0),
            queue_depth_p4_video: AtomicUsize::new(0),
            last_uia_latency_us: AtomicU64::new(0),
            max_uia_latency_us: AtomicU64::new(0),
            last_screenshot_latency_us: AtomicU64::new(0),
            last_disk_write_latency_us: AtomicU64::new(0),
            video_encoder_fps: AtomicU32::new(0),
            memory_working_set_bytes: AtomicU64::new(0),
            cpu_usage_pct_x100: AtomicU32::new(0),
            pending_upload_bytes: AtomicU64::new(0),
            upload_throughput_bps: AtomicU64::new(0),
            failed_chunks_total: AtomicU64::new(0),
            rate_state: RwLock::new(RateCalculationState {
                last_sample_instant: Instant::now(),
                last_events_captured: 0,
                last_events_dropped: 0,
                last_bytes_uploaded: 0,
            }),
        })
    }

    #[inline]
    pub fn record_event_captured(&self) {
        self.events_captured_total.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn record_event_dropped(&self) {
        self.events_dropped_total.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn record_canonical_action(&self) {
        self.canonical_actions_total.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn record_uia_latency(&self, duration_us: u64) {
        self.last_uia_latency_us.store(duration_us, Ordering::Relaxed);
        self.max_uia_latency_us.fetch_max(duration_us, Ordering::Relaxed);
    }

    #[inline]
    pub fn record_screenshot_latency(&self, duration_us: u64) {
        self.last_screenshot_latency_us.store(duration_us, Ordering::Relaxed);
    }

    #[inline]
    pub fn record_disk_write_latency(&self, duration_us: u64) {
        self.last_disk_write_latency_us.store(duration_us, Ordering::Relaxed);
    }

    #[inline]
    pub fn update_queue_depths(&self, p0: usize, p1: usize, p2: usize, p3: usize, p4: usize) {
        self.queue_depth_p0_input.store(p0, Ordering::Relaxed);
        self.queue_depth_p1_window.store(p1, Ordering::Relaxed);
        self.queue_depth_p2_dom_uia.store(p2, Ordering::Relaxed);
        self.queue_depth_p3_screenshot.store(p3, Ordering::Relaxed);
        self.queue_depth_p4_video.store(p4, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        let current_captured = self.events_captured_total.load(Ordering::Relaxed);
        let current_dropped = self.events_dropped_total.load(Ordering::Relaxed);
        let now = Instant::now();

        let (events_per_sec, dropped_per_sec) = {
            let mut state = self.rate_state.write();
            let elapsed_secs = now.duration_since(state.last_sample_instant).as_secs_f64();
            if elapsed_secs >= 0.001 {
                let d_captured = current_captured.saturating_sub(state.last_events_captured);
                let d_dropped = current_dropped.saturating_sub(state.last_events_dropped);
                
                state.last_sample_instant = now;
                state.last_events_captured = current_captured;
                state.last_events_dropped = current_dropped;

                (d_captured as f64 / elapsed_secs, d_dropped as f64 / elapsed_secs)
            } else {
                (0.0, 0.0)
            }
        };

        MetricsSnapshot {
            timestamp_utc: chrono::Utc::now(),
            events_per_sec,
            dropped_per_sec,
            events_captured_total: current_captured,
            events_dropped_total: current_dropped,
            canonical_actions_total: self.canonical_actions_total.load(Ordering::Relaxed),
            queue_depths: QueueDepths {
                p0_input: self.queue_depth_p0_input.load(Ordering::Relaxed),
                p1_window: self.queue_depth_p1_window.load(Ordering::Relaxed),
                p2_dom_uia: self.queue_depth_p2_dom_uia.load(Ordering::Relaxed),
                p3_screenshot: self.queue_depth_p3_screenshot.load(Ordering::Relaxed),
                p4_video: self.queue_depth_p4_video.load(Ordering::Relaxed),
            },
            latencies_us: LatencyMetrics {
                last_uia_latency_us: self.last_uia_latency_us.load(Ordering::Relaxed),
                max_uia_latency_us: self.max_uia_latency_us.load(Ordering::Relaxed),
                last_screenshot_latency_us: self.last_screenshot_latency_us.load(Ordering::Relaxed),
                last_disk_write_latency_us: self.last_disk_write_latency_us.load(Ordering::Relaxed),
            },
            video_fps: self.video_encoder_fps.load(Ordering::Relaxed),
            memory_working_set_bytes: self.memory_working_set_bytes.load(Ordering::Relaxed),
            cpu_usage_pct: self.cpu_usage_pct_x100.load(Ordering::Relaxed) as f32 / 100.0,
            pending_upload_bytes: self.pending_upload_bytes.load(Ordering::Relaxed),
            upload_throughput_bytes_sec: self.upload_throughput_bps.load(Ordering::Relaxed),
            failed_chunks_total: self.failed_chunks_total.load(Ordering::Relaxed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector_atomic_increments() {
        let metrics = MetricsCollector::new();
        for _ in 0..1000 {
            metrics.record_event_captured();
        }
        for _ in 0..5 {
            metrics.record_event_dropped();
        }
        metrics.record_uia_latency(1500);

        let snap = metrics.snapshot();
        assert_eq!(snap.events_captured_total, 1000);
        assert_eq!(snap.events_dropped_total, 5);
        assert_eq!(snap.latencies_us.last_uia_latency_us, 1500);
        assert_eq!(snap.latencies_us.max_uia_latency_us, 1500);
    }
}
