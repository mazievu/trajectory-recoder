use crate::metrics::{EventBusMetrics, EventBusMetricsSnapshot};
use crate::priority::{Priority, PriorityShedMode};
use core_types::event::RawEvent;
use crossbeam_channel::{Receiver, Select, Sender, TryRecvError, TrySendError};
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

/// Error types that can occur during event bus operations.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum EventBusError {
    #[error("Channel disconnected")]
    Disconnected,
    #[error("Channel full for critical priority {0:?}")]
    QueueFull(Priority),
    #[error("Timeout while receiving event")]
    Timeout,
    #[error("Queue empty")]
    Empty,
}

/// Result of a publish attempt on the event bus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishResult {
    /// Event successfully enqueued.
    Published,
    /// Event was dropped according to the priority shedding policy or channel saturation.
    Dropped(Priority),
}

/// Configuration options for initializing the EventBus.
#[derive(Debug, Clone)]
pub struct EventBusConfig {
    pub capacities: [usize; 5],
    pub initial_shed_mode: PriorityShedMode,
}

impl Default for EventBusConfig {
    fn default() -> Self {
        Self {
            capacities: [
                Priority::P0_Input.default_capacity(),
                Priority::P1_Window.default_capacity(),
                Priority::P2_DomUia.default_capacity(),
                Priority::P3_Screenshot.default_capacity(),
                Priority::P4_Video.default_capacity(),
            ],
            initial_shed_mode: PriorityShedMode::Normal,
        }
    }
}

impl EventBusConfig {
    /// Create a configuration with custom queue capacities.
    pub fn with_capacities(p0: usize, p1: usize, p2: usize, p3: usize, p4: usize) -> Self {
        Self {
            capacities: [p0, p1, p2, p3, p4],
            initial_shed_mode: PriorityShedMode::Normal,
        }
    }
}

/// Bounded multi-producer multi-consumer priority event bus.
pub struct EventBus<T = RawEvent> {
    senders: [Sender<T>; 5],
    receivers: [Receiver<T>; 5],
    metrics: Arc<EventBusMetrics>,
    shed_mode: Arc<RwLock<PriorityShedMode>>,
}

impl<T> Default for EventBus<T> {
    fn default() -> Self {
        Self::new(EventBusConfig::default())
    }
}

impl<T> EventBus<T> {
    /// Create a new `EventBus` with given configuration.
    pub fn new(config: EventBusConfig) -> Self {
        let (s0, r0) = crossbeam_channel::bounded(config.capacities[0]);
        let (s1, r1) = crossbeam_channel::bounded(config.capacities[1]);
        let (s2, r2) = crossbeam_channel::bounded(config.capacities[2]);
        let (s3, r3) = crossbeam_channel::bounded(config.capacities[3]);
        let (s4, r4) = crossbeam_channel::bounded(config.capacities[4]);

        Self {
            senders: [s0, s1, s2, s3, s4],
            receivers: [r0, r1, r2, r3, r4],
            metrics: Arc::new(EventBusMetrics::new()),
            shed_mode: Arc::new(RwLock::new(config.initial_shed_mode)),
        }
    }

    /// Set the dynamic priority shedding mode (e.g. under disk backpressure).
    pub fn set_shed_mode(&self, mode: PriorityShedMode) {
        *self.shed_mode.write() = mode;
    }

    /// Get current shedding mode.
    pub fn shed_mode(&self) -> PriorityShedMode {
        *self.shed_mode.read()
    }

    /// Create a cloneable handle for publishing events from multiple producers.
    pub fn publisher(&self) -> EventPublisher<T> {
        EventPublisher {
            senders: self.senders.clone(),
            metrics: self.metrics.clone(),
            shed_mode: self.shed_mode.clone(),
        }
    }

    /// Create a cloneable receiver handle for consuming events by priority.
    pub fn receiver(&self) -> EventReceiver<T> {
        EventReceiver {
            receivers: self.receivers.clone(),
            metrics: self.metrics.clone(),
        }
    }

    /// Publish an item with explicit priority.
    pub fn publish(&self, priority: Priority, item: T) -> Result<PublishResult, EventBusError> {
        let shed_mode = *self.shed_mode.read();
        if shed_mode.should_shed(priority) {
            self.metrics.record_dropped(priority);
            return Ok(PublishResult::Dropped(priority));
        }

        let sender = &self.senders[priority as usize];
        match sender.try_send(item) {
            Ok(()) => {
                self.metrics.record_published(priority);
                Ok(PublishResult::Published)
            }
            Err(TrySendError::Full(dropped_item)) => {
                if priority.is_critical() {
                    // For critical P0/P1 events, wait briefly or block to prevent data loss
                    match sender.send_timeout(dropped_item, Duration::from_millis(500)) {
                        Ok(()) => {
                            self.metrics.record_published(priority);
                            Ok(PublishResult::Published)
                        }
                        Err(_) => {
                            self.metrics.record_dropped(priority);
                            Err(EventBusError::QueueFull(priority))
                        }
                    }
                } else {
                    // Non-critical events shed on saturation
                    self.metrics.record_dropped(priority);
                    Ok(PublishResult::Dropped(priority))
                }
            }
            Err(TrySendError::Disconnected(_)) => Err(EventBusError::Disconnected),
        }
    }

    /// Get current metrics snapshot.
    pub fn metrics(&self) -> EventBusMetricsSnapshot {
        let depths = [
            self.receivers[0].len(),
            self.receivers[1].len(),
            self.receivers[2].len(),
            self.receivers[3].len(),
            self.receivers[4].len(),
        ];
        self.metrics.snapshot(depths)
    }

    /// Check total number of queued events across all priorities.
    pub fn total_queue_depth(&self) -> usize {
        self.receivers.iter().map(|r| r.len()).sum()
    }
}

impl EventBus<RawEvent> {
    /// Helper to publish a `RawEvent` by automatically determining its priority.
    pub fn publish_event(&self, event: RawEvent) -> Result<PublishResult, EventBusError> {
        let priority = Priority::from_raw_payload(&event.payload);
        self.publish(priority, event)
    }
}

/// Cloneable handle for producing events into the event bus.
#[derive(Clone)]
pub struct EventPublisher<T> {
    senders: [Sender<T>; 5],
    metrics: Arc<EventBusMetrics>,
    shed_mode: Arc<RwLock<PriorityShedMode>>,
}

impl<T> EventPublisher<T> {
    /// Publish an item with explicit priority.
    pub fn publish(&self, priority: Priority, item: T) -> Result<PublishResult, EventBusError> {
        let shed_mode = *self.shed_mode.read();
        if shed_mode.should_shed(priority) {
            self.metrics.record_dropped(priority);
            return Ok(PublishResult::Dropped(priority));
        }

        let sender = &self.senders[priority as usize];
        match sender.try_send(item) {
            Ok(()) => {
                self.metrics.record_published(priority);
                Ok(PublishResult::Published)
            }
            Err(TrySendError::Full(dropped_item)) => {
                if priority.is_critical() {
                    match sender.send_timeout(dropped_item, Duration::from_millis(500)) {
                        Ok(()) => {
                            self.metrics.record_published(priority);
                            Ok(PublishResult::Published)
                        }
                        Err(_) => {
                            self.metrics.record_dropped(priority);
                            Err(EventBusError::QueueFull(priority))
                        }
                    }
                } else {
                    self.metrics.record_dropped(priority);
                    Ok(PublishResult::Dropped(priority))
                }
            }
            Err(TrySendError::Disconnected(_)) => Err(EventBusError::Disconnected),
        }
    }
}

impl EventPublisher<RawEvent> {
    /// Helper to publish a `RawEvent` by automatically determining its priority.
    pub fn publish_event(&self, event: RawEvent) -> Result<PublishResult, EventBusError> {
        let priority = Priority::from_raw_payload(&event.payload);
        self.publish(priority, event)
    }
}

/// Cloneable receiver handle for consuming events by priority (P0 -> P1 -> P2 -> P3 -> P4).
#[derive(Clone)]
pub struct EventReceiver<T> {
    receivers: [Receiver<T>; 5],
    metrics: Arc<EventBusMetrics>,
}

impl<T> EventReceiver<T> {
    /// Non-blocking receive checking queues strictly in priority order: P0 -> P1 -> P2 -> P3 -> P4.
    pub fn try_recv(&self) -> Result<(Priority, T), EventBusError> {
        for &priority in &Priority::ALL {
            let receiver = &self.receivers[priority as usize];
            match receiver.try_recv() {
                Ok(item) => {
                    self.metrics.record_consumed();
                    return Ok((priority, item));
                }
                Err(TryRecvError::Empty) => continue,
                Err(TryRecvError::Disconnected) => {
                    // Check if all are disconnected
                    if self.all_disconnected() {
                        return Err(EventBusError::Disconnected);
                    }
                }
            }
        }
        Err(EventBusError::Empty)
    }

    /// Blocking receive that selects the highest priority event available, or waits until one arrives.
    pub fn recv(&self) -> Result<(Priority, T), EventBusError> {
        // First try non-blocking sweep to strictly enforce priority ordering
        if let Ok(item) = self.try_recv() {
            return Ok(item);
        }

        // If all queues are empty, wait with crossbeam Select
        let mut sel = Select::new();
        let idx0 = sel.recv(&self.receivers[0]);
        let idx1 = sel.recv(&self.receivers[1]);
        let idx2 = sel.recv(&self.receivers[2]);
        let idx3 = sel.recv(&self.receivers[3]);
        let idx4 = sel.recv(&self.receivers[4]);

        let oper = sel.select();
        let index = oper.index();

        let (priority, item) = if index == idx0 {
            (Priority::P0_Input, oper.recv(&self.receivers[0]))
        } else if index == idx1 {
            (Priority::P1_Window, oper.recv(&self.receivers[1]))
        } else if index == idx2 {
            (Priority::P2_DomUia, oper.recv(&self.receivers[2]))
        } else if index == idx3 {
            (Priority::P3_Screenshot, oper.recv(&self.receivers[3]))
        } else if index == idx4 {
            (Priority::P4_Video, oper.recv(&self.receivers[4]))
        } else {
            return Err(EventBusError::Disconnected);
        };

        match item {
            Ok(val) => {
                self.metrics.record_consumed();
                Ok((priority, val))
            }
            Err(_) => {
                if self.all_disconnected() {
                    Err(EventBusError::Disconnected)
                } else {
                    self.recv()
                }
            }
        }
    }

    /// Blocking receive with a timeout.
    pub fn recv_timeout(&self, timeout: Duration) -> Result<(Priority, T), EventBusError> {
        // First try non-blocking
        if let Ok(item) = self.try_recv() {
            return Ok(item);
        }

        let mut sel = Select::new();
        let idx0 = sel.recv(&self.receivers[0]);
        let idx1 = sel.recv(&self.receivers[1]);
        let idx2 = sel.recv(&self.receivers[2]);
        let idx3 = sel.recv(&self.receivers[3]);
        let idx4 = sel.recv(&self.receivers[4]);

        let oper = match sel.select_timeout(timeout) {
            Ok(oper) => oper,
            Err(_) => return Err(EventBusError::Timeout),
        };

        let index = oper.index();
        let (priority, item) = if index == idx0 {
            (Priority::P0_Input, oper.recv(&self.receivers[0]))
        } else if index == idx1 {
            (Priority::P1_Window, oper.recv(&self.receivers[1]))
        } else if index == idx2 {
            (Priority::P2_DomUia, oper.recv(&self.receivers[2]))
        } else if index == idx3 {
            (Priority::P3_Screenshot, oper.recv(&self.receivers[3]))
        } else if index == idx4 {
            (Priority::P4_Video, oper.recv(&self.receivers[4]))
        } else {
            return Err(EventBusError::Disconnected);
        };

        match item {
            Ok(val) => {
                self.metrics.record_consumed();
                Ok((priority, val))
            }
            Err(_) => Err(EventBusError::Disconnected),
        }
    }

    fn all_disconnected(&self) -> bool {
        self.receivers.iter().all(|r| r.is_empty())
    }
}
