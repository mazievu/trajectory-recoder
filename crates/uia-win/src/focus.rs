//! Bounded, value-free evidence of changes to the focused UI Automation target.
//!
//! Native UIA event handlers require a message-pumped COM apartment and can make the
//! recorder unavailable when a provider blocks. This detector deliberately works with
//! the existing time-bounded `inspect_focused` query: callers poll it only at semantic
//! boundaries (foreground change, click, or keyboard activity), then persist an event
//! only when the focused control changed.

use core_types::metadata::{AncestorElementMetadata, TargetMetadata};
use serde::{Deserialize, Serialize};

/// Maximum number of Unicode scalar values retained for a focus metadata field.
pub const MAX_FOCUS_METADATA_CHARS: usize = 256;

/// A semantic transition in UI Automation keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FocusChange {
    Gained,
    Changed,
    Lost,
}

/// Persistable evidence of one focus transition.
///
/// `target` is absent only for `Lost`; both targets are always value-free and bounded.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FocusEvidence {
    pub change: FocusChange,
    pub previous: Option<TargetMetadata>,
    pub target: Option<TargetMetadata>,
}

/// State held by a recorder while it samples the focused control.
#[derive(Debug, Clone, Default)]
pub struct FocusChangeDetector {
    previous: Option<TrackedFocus>,
}

#[derive(Debug, Clone)]
struct TrackedFocus {
    identity: FocusIdentity,
    target: TargetMetadata,
}

/// Stable identifiers used to decide whether focus moved to another control.
///
/// Values and help text are deliberately excluded: they may change while an editable
/// control remains focused, and retaining them would turn a focus detector into a
/// content recorder.
#[derive(Debug, Clone, PartialEq, Eq)]
struct FocusIdentity {
    name: Option<String>,
    control_type: Option<String>,
    automation_id: Option<String>,
    class_name: Option<String>,
    framework_id: Option<String>,
    bounding_rect: Option<core_types::metadata::BoundingRect>,
}

impl FocusChangeDetector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Observe one focused-element sample and return evidence only for a transition.
    ///
    /// The input is sanitized before it is stored or returned, including when it is
    /// supplied by a mock or a third-party UIA provider.
    pub fn observe(&mut self, target: Option<TargetMetadata>) -> Option<FocusEvidence> {
        let current = target.map(sanitize_target).map(TrackedFocus::new);

        match (&self.previous, current) {
            (None, None) => None,
            (None, Some(current)) => {
                let evidence = FocusEvidence {
                    change: FocusChange::Gained,
                    previous: None,
                    target: Some(current.target.clone()),
                };
                self.previous = Some(current);
                Some(evidence)
            }
            (Some(previous), None) => {
                let evidence = FocusEvidence {
                    change: FocusChange::Lost,
                    previous: Some(previous.target.clone()),
                    target: None,
                };
                self.previous = None;
                Some(evidence)
            }
            (Some(previous), Some(current)) if previous.identity == current.identity => None,
            (Some(previous), Some(current)) => {
                let evidence = FocusEvidence {
                    change: FocusChange::Changed,
                    previous: Some(previous.target.clone()),
                    target: Some(current.target.clone()),
                };
                self.previous = Some(current);
                Some(evidence)
            }
        }
    }

    /// Forget the last observation, for example when a recording session ends.
    pub fn reset(&mut self) {
        self.previous = None;
    }
}

impl TrackedFocus {
    fn new(target: TargetMetadata) -> Self {
        let identity = FocusIdentity::from(&target);
        Self { identity, target }
    }
}

impl From<&TargetMetadata> for FocusIdentity {
    fn from(target: &TargetMetadata) -> Self {
        Self {
            name: target.name.clone(),
            control_type: target.control_type.clone(),
            automation_id: target.automation_id.clone(),
            class_name: target.class_name.clone(),
            framework_id: target.framework_id.clone(),
            bounding_rect: target.bounding_rect,
        }
    }
}

fn sanitize_target(mut target: TargetMetadata) -> TargetMetadata {
    target.name = bound(target.name);
    target.control_type = bound(target.control_type);
    target.automation_id = bound(target.automation_id);
    target.class_name = bound(target.class_name);
    target.framework_id = bound(target.framework_id);
    target.help_text = bound(target.help_text);

    // Focus evidence identifies controls; it must never retain a field's content.
    target.value = None;

    target.ancestor_chain = target
        .ancestor_chain
        .into_iter()
        .take(3)
        .map(sanitize_ancestor)
        .collect();
    target.ancestors.clear();
    target.dom_selector = None;
    target.xpath = None;
    target
}

fn sanitize_ancestor(mut ancestor: AncestorElementMetadata) -> AncestorElementMetadata {
    ancestor.name = bound(ancestor.name);
    ancestor.control_type = bound(ancestor.control_type);
    ancestor.automation_id = bound(ancestor.automation_id);
    ancestor.class_name = bound(ancestor.class_name);
    ancestor.framework_id = bound(ancestor.framework_id);
    ancestor
}

fn bound(value: Option<String>) -> Option<String> {
    value.map(|value| value.chars().take(MAX_FOCUS_METADATA_CHARS).collect())
}
