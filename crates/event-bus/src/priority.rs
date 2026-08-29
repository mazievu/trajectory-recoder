use core_types::event::{EventSource, RawEventPayload};
use serde::{Deserialize, Serialize};

/// Priority levels for event routing in the Event Bus.
///
/// Order of priority:
/// - P0: Input / User Interaction (Mouse, Keyboard, Direct Actions) — CRITICAL, NEVER DROPPED
/// - P1: Window / System State (Focus, Lifecycle, Topology, System Power) — HIGH, NEVER DROPPED
/// - P2: DOM / UI Automation (Browser DOM, UIA Tree Walker) — MEDIUM
/// - P3: Screenshot / Visual Diff (WebP frames, Perceptual Diffs) — LOW, SHED FIRST UNDER PRESSURE
/// - P4: Video / Audio Stream (H.264 video fragments) — BACKGROUND, SHED FIRST UNDER SATURATION
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Priority {
    /// P0: Mouse, Keyboard, Raw Input, and immediate user actions.
    P0_Input = 0,
    /// P1: Window state changes, system power, monitor topology.
    P1_Window = 1,
    /// P2: UI Automation queries and browser DOM events.
    P2_DomUia = 2,
    /// P3: Screenshots, screen captures, visual diffs.
    P3_Screenshot = 3,
    /// P4: Continuous video fragments.
    P4_Video = 4,
}

impl Priority {
    /// All priorities in order from highest (P0) to lowest (P4).
    pub const ALL: [Priority; 5] = [
        Priority::P0_Input,
        Priority::P1_Window,
        Priority::P2_DomUia,
        Priority::P3_Screenshot,
        Priority::P4_Video,
    ];

    /// Default bounded queue capacity per priority level according to Master Spec.
    pub const fn default_capacity(&self) -> usize {
        match self {
            Priority::P0_Input => 50_000,
            Priority::P1_Window => 10_000,
            Priority::P2_DomUia => 5_000,
            Priority::P3_Screenshot => 1_000,
            Priority::P4_Video => 200,
        }
    }

    /// Whether this priority level is critical and must never be dropped under normal backpressure.
    pub const fn is_critical(&self) -> bool {
        matches!(self, Priority::P0_Input | Priority::P1_Window)
    }

    /// Determine priority from an `EventSource`.
    pub fn from_event_source(source: EventSource) -> Self {
        match source {
            EventSource::Win32Hook | EventSource::InputHook => Priority::P0_Input,
            EventSource::WinEvent | EventSource::SystemTelemetry | EventSource::SessionRouter => {
                Priority::P1_Window
            }
            EventSource::UiAutomation | EventSource::BrowserExtension => Priority::P2_DomUia,
            EventSource::WgcScreenCapture => Priority::P3_Screenshot,
            EventSource::ClipboardListener | EventSource::FileWatcher => Priority::P1_Window,
        }
    }

    /// Determine priority from a `RawEventPayload`.
    pub fn from_raw_payload(payload: &RawEventPayload) -> Self {
        match payload {
            RawEventPayload::Mouse(_) | RawEventPayload::Keyboard(_) => Priority::P0_Input,
            RawEventPayload::Window(_)
            | RawEventPayload::Clipboard(_)
            | RawEventPayload::File(_)
            | RawEventPayload::System(_)
            | RawEventPayload::Session(_) => Priority::P1_Window,
            RawEventPayload::UiAutomation(_) | RawEventPayload::Browser(_) => Priority::P2_DomUia,
            RawEventPayload::Screen(_) => Priority::P3_Screenshot,
        }
    }
}

/// Dynamic shedding level to shed lower priority queues during backpressure or disk pressure.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub enum PriorityShedMode {
    /// Normal operation: No shedding unless channel capacity is exceeded.
    #[default]
    Normal,
    /// Shed P4 (Video) events immediately to relieve pressure.
    ShedP4,
    /// Shed P4 (Video) and P3 (Screenshots) events to preserve semantic and input events.
    ShedP4AndP3,
    /// Shed P4, P3, and P2 events (only capture raw inputs and window states).
    ShedP4P3P2,
}

impl PriorityShedMode {
    /// Check if a given priority should be dropped under this shedding mode.
    pub fn should_shed(&self, priority: Priority) -> bool {
        match self {
            PriorityShedMode::Normal => false,
            PriorityShedMode::ShedP4 => priority >= Priority::P4_Video,
            PriorityShedMode::ShedP4AndP3 => priority >= Priority::P3_Screenshot,
            PriorityShedMode::ShedP4P3P2 => priority >= Priority::P2_DomUia,
        }
    }
}
