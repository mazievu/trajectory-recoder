//! High-throughput bounded MPMC event bus with priority routing and shedding.
//!
//! Designed for the Trajectory Recorder to support 24/7 capture streams with
//! zero data loss on critical user input (P0) and active window changes (P1),
//! while safely shedding high-volume background streams (P4 video, P3 screenshot)
//! during resource saturation or disk backpressure.

pub mod bus;
pub mod metrics;
pub mod priority;

pub use bus::{
    EventBus, EventBusConfig, EventBusError, EventPublisher, EventReceiver, PublishResult,
};
pub use metrics::{EventBusMetrics, EventBusMetricsSnapshot};
pub use priority::{Priority, PriorityShedMode};

#[cfg(test)]
mod tests {
    use super::*;
    use core_types::event::{EventSource, RawEvent, RawEventPayload, RawMouseEvent};
    use core_types::id::GlobalEventId;
    use core_types::timestamp::DualTimestamp;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;
    use std::time::Duration;

    fn sample_raw_event(id: u64, payload: RawEventPayload) -> RawEvent {
        RawEvent::new(
            id,
            GlobalEventId::new(id),
            DualTimestamp::from_parts(chrono::Utc::now(), id * 1000, 0),
            "test_machine".to_string(),
            1,
            "test_user".to_string(),
            EventSource::InputHook,
            id,
            payload,
        )
    }

    #[test]
    fn test_priority_ordering() {
        let bus = EventBus::<String>::new(EventBusConfig::default());

        // Publish in reverse priority order: P4, P3, P2, P1, P0
        bus.publish(Priority::P4_Video, "video_frame".into())
            .unwrap();
        bus.publish(Priority::P3_Screenshot, "screenshot".into())
            .unwrap();
        bus.publish(Priority::P2_DomUia, "dom_click".into())
            .unwrap();
        bus.publish(Priority::P1_Window, "window_focus".into())
            .unwrap();
        bus.publish(Priority::P0_Input, "mouse_click".into())
            .unwrap();

        let receiver = bus.receiver();

        // Must receive strictly in priority order: P0 -> P1 -> P2 -> P3 -> P4
        let (p0, v0) = receiver.try_recv().unwrap();
        assert_eq!(p0, Priority::P0_Input);
        assert_eq!(v0, "mouse_click");

        let (p1, v1) = receiver.try_recv().unwrap();
        assert_eq!(p1, Priority::P1_Window);
        assert_eq!(v1, "window_focus");

        let (p2, v2) = receiver.try_recv().unwrap();
        assert_eq!(p2, Priority::P2_DomUia);
        assert_eq!(v2, "dom_click");

        let (p3, v3) = receiver.try_recv().unwrap();
        assert_eq!(p3, Priority::P3_Screenshot);
        assert_eq!(v3, "screenshot");

        let (p4, v4) = receiver.try_recv().unwrap();
        assert_eq!(p4, Priority::P4_Video);
        assert_eq!(v4, "video_frame");

        assert_eq!(receiver.try_recv().unwrap_err(), EventBusError::Empty);
    }

    #[test]
    fn test_saturation_shedding_p4_and_p3() {
        // Small capacities to trigger saturation shedding
        let config = EventBusConfig::with_capacities(10, 10, 10, 2, 2);
        let bus = EventBus::<String>::new(config);

        // Fill P4
        assert_eq!(
            bus.publish(Priority::P4_Video, "v1".into()).unwrap(),
            PublishResult::Published
        );
        assert_eq!(
            bus.publish(Priority::P4_Video, "v2".into()).unwrap(),
            PublishResult::Published
        );
        // Saturated P4 must be dropped
        assert_eq!(
            bus.publish(Priority::P4_Video, "v3".into()).unwrap(),
            PublishResult::Dropped(Priority::P4_Video)
        );

        // Fill P3
        assert_eq!(
            bus.publish(Priority::P3_Screenshot, "s1".into()).unwrap(),
            PublishResult::Published
        );
        assert_eq!(
            bus.publish(Priority::P3_Screenshot, "s2".into()).unwrap(),
            PublishResult::Published
        );
        // Saturated P3 must be dropped
        assert_eq!(
            bus.publish(Priority::P3_Screenshot, "s3".into()).unwrap(),
            PublishResult::Dropped(Priority::P3_Screenshot)
        );

        // P0 must still publish normally
        assert_eq!(
            bus.publish(Priority::P0_Input, "i1".into()).unwrap(),
            PublishResult::Published
        );

        let metrics = bus.metrics();
        assert_eq!(metrics.published_p4, 2);
        assert_eq!(metrics.dropped_p4, 1);
        assert_eq!(metrics.published_p3, 2);
        assert_eq!(metrics.dropped_p3, 1);
        assert_eq!(metrics.published_p0, 1);
        assert_eq!(metrics.dropped_p0, 0);
    }

    #[test]
    fn test_dynamic_shed_mode() {
        let bus = EventBus::<String>::new(EventBusConfig::default());

        bus.set_shed_mode(PriorityShedMode::ShedP4AndP3);
        assert_eq!(
            bus.publish(Priority::P4_Video, "v1".into()).unwrap(),
            PublishResult::Dropped(Priority::P4_Video)
        );
        assert_eq!(
            bus.publish(Priority::P3_Screenshot, "s1".into()).unwrap(),
            PublishResult::Dropped(Priority::P3_Screenshot)
        );
        assert_eq!(
            bus.publish(Priority::P0_Input, "i1".into()).unwrap(),
            PublishResult::Published
        );

        let metrics = bus.metrics();
        assert_eq!(metrics.dropped_p4, 1);
        assert_eq!(metrics.dropped_p3, 1);
        assert_eq!(metrics.published_p0, 1);
    }

    #[test]
    fn test_raw_event_publishing() {
        let bus = EventBus::new(EventBusConfig::default());
        let event = sample_raw_event(1, RawEventPayload::Mouse(RawMouseEvent::default()));

        let res = bus.publish_event(event.clone()).unwrap();
        assert_eq!(res, PublishResult::Published);

        let receiver = bus.receiver();
        let (p, received) = receiver.try_recv().unwrap();
        assert_eq!(p, Priority::P0_Input);
        assert_eq!(received.event_id, 1);
    }

    #[test]
    fn test_multithreaded_producers_and_consumer() {
        let bus = Arc::new(EventBus::<u64>::new(EventBusConfig::default()));
        let mut handles = Vec::new();
        let count_per_thread = 500;

        // Spawn 4 producer threads
        for t in 0..4 {
            let b = bus.clone();
            handles.push(thread::spawn(move || {
                let publisher = b.publisher();
                let priority = match t {
                    0 => Priority::P0_Input,
                    1 => Priority::P1_Window,
                    2 => Priority::P2_DomUia,
                    _ => Priority::P3_Screenshot,
                };
                for i in 0..count_per_thread {
                    publisher.publish(priority, (t * 1000 + i) as u64).unwrap();
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        let receiver = bus.receiver();
        let total_received = Arc::new(AtomicUsize::new(0));
        let expected_total = 4 * count_per_thread;

        while total_received.load(Ordering::SeqCst) < expected_total {
            match receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(_) => {
                    total_received.fetch_add(1, Ordering::SeqCst);
                }
                Err(e) => {
                    panic!("Unexpected receive error: {:?}", e);
                }
            }
        }

        assert_eq!(total_received.load(Ordering::SeqCst), expected_total);
        let metrics = bus.metrics();
        assert_eq!(metrics.total_published, expected_total as u64);
        assert_eq!(metrics.total_consumed, expected_total as u64);
    }
}
