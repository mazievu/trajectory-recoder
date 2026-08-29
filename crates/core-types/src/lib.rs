//! Pure Rust core types, schemas, and identifiers for Trajectory Recorder.
//!
//! STRICT INVARIANT: This crate must NEVER depend on Windows APIs (`windows`, `winapi`)
//! or platform-specific system calls. It is pure data schemas and business invariants.

pub mod action;
pub mod error;
pub mod event;
pub mod id;
pub mod metadata;
pub mod state;
pub mod timestamp;

pub use action::{
    ActionDetail, ActionParameters, ActionType, CanonicalAction, CanonicalActionBuilder,
    ClickParams, ClipboardParams, DialogParams, DragDropParams, FileOperationParams,
    KeyPressParams, NavigationParams, ScrollParams, ShortcutParams, SystemStateParams,
    TypeTextParams, UnknownParams, WaitParams, WindowLifecycleParams,
};
pub use error::{ErrorSeverity, ErrorTaxonomy};
pub use event::{
    EventSource, RawBrowserEvent, RawClipboardEvent, RawEvent, RawEventPayload, RawFileEvent,
    RawKeyboardEvent, RawMouseEvent, RawScreenEvent, RawSessionEvent, RawSystemEvent, RawUiaEvent,
    RawWindowEvent,
};
pub use id::{GlobalEventId, MachineId, SessionEventId, SessionId, UserId};
pub use metadata::{
    AncestorElementMetadata, ApplicationContext, BoundingBox, BoundingRect, BrowserContext,
    ContextMetadata, DisplayContext, DomSelectorMetadata, ModifierState, MouseButton, Point2D,
    ScrollDirection, TargetMetadata, WindowContext,
};
pub use state::{
    ActionEvidence, ScreenshotRef, ScreenshotTrigger, StateChange, StateChangeKind, StateEvidence,
    StateSnapshot, UiStateSnapshot, VideoTimeRange,
};
pub use timestamp::{DualTimestamp, EventTimestamp};

pub const SCHEMA_IDENTIFIER: &str = "gtf.trajectory";
pub const SCHEMA_VERSION: &str = "1.0";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_constants() {
        assert_eq!(SCHEMA_IDENTIFIER, "gtf.trajectory");
        assert_eq!(SCHEMA_VERSION, "1.0");
    }
}
