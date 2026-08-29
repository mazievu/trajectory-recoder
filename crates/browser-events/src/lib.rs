//! Browser extension DOM schemas, mutation models, and event definitions.

use core_types::event::{EventSource, RawBrowserEvent, RawEvent, RawEventPayload};
use core_types::id::GlobalEventId;
use core_types::metadata::{DomSelectorMetadata, TargetMetadata};
use core_types::timestamp::DualTimestamp;
use serde::{Deserialize, Serialize};

/// Detailed browser DOM event payload sent from Chrome/Edge Manifest V3 extension.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrowserDomEvent {
    pub tab_id: u32,
    pub url: String,
    pub page_title: String,
    pub event_type: String, // "CLICK", "INPUT", "CHANGE", "SUBMIT", "MUTATION", "SPA_NAVIGATION"
    pub tag: String,
    pub role: Option<String>,
    pub visible_text: Option<String>,
    pub aria_label: Option<String>,
    pub element_id: Option<String>,
    pub class_name: Option<String>,
    pub href: Option<String>,
    pub placeholder: Option<String>,
    pub input_type: Option<String>,
    pub value: Option<String>,
    pub css_selector: Option<String>,
    pub xpath: Option<String>,
    pub timestamp_ms: u64,
    pub is_password: bool,
    #[serde(default)]
    pub mutation_info: Option<DomMutationInfo>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DomMutationInfo {
    pub mutation_type: String, // "childList", "attributes"
    pub added_nodes_count: usize,
    pub removed_nodes_count: usize,
    pub attribute_name: Option<String>,
    pub target_summary: Option<String>,
}

impl BrowserDomEvent {
    /// Convert to `TargetMetadata` for canonical action association.
    pub fn to_target_metadata(&self) -> TargetMetadata {
        let selector = DomSelectorMetadata {
            tag: self.tag.clone(),
            role: self.role.clone(),
            visible_text: self.visible_text.clone(),
            aria_label: self.aria_label.clone(),
            id: self.element_id.clone(),
            class: self.class_name.clone(),
            href: self.href.clone(),
            placeholder: self.placeholder.clone(),
            input_type: self.input_type.clone(),
            css_selector: self.css_selector.clone(),
            xpath: self.xpath.clone(),
        };

        TargetMetadata {
            name: self.visible_text.clone().or(self.aria_label.clone()),
            control_type: Some(self.tag.clone()),
            automation_id: self.element_id.clone(),
            class_name: self.class_name.clone(),
            framework_id: Some("DOM".to_string()),
            bounding_rect: None,
            bounding_box: None,
            is_enabled: Some(true),
            is_keyboard_focusable: Some(true),
            is_password: self.is_password,
            value: if self.is_password {
                Some("[PASSWORD_REDACTED]".to_string())
            } else {
                self.value.clone()
            },
            help_text: None,
            ancestor_chain: Vec::new(),
            ancestors: Vec::new(),
            dom_selector: Some(selector),
            xpath: self.xpath.clone(),
        }
    }

    /// Convert to `RawEvent` with `RawBrowserEvent` payload.
    pub fn to_raw_event(
        &self,
        event_seq: u64,
        global_event_id: GlobalEventId,
        machine_id: impl Into<String>,
        windows_session_id: u32,
        user_id: impl Into<String>,
    ) -> RawEvent {
        let payload = RawEventPayload::Browser(RawBrowserEvent {
            tab_id: self.tab_id,
            event_type: self.event_type.clone(),
            url: self.url.clone(),
            tag_name: self.tag.clone(),
            target_id: self.element_id.clone(),
            target_class: self.class_name.clone(),
            target_text: self.visible_text.clone(),
            css_selector: self.css_selector.clone(),
            xpath: self.xpath.clone(),
            bounds: Default::default(),
        });

        RawEvent::new(
            event_seq,
            global_event_id,
            DualTimestamp::now(),
            machine_id.into(),
            windows_session_id,
            user_id.into(),
            EventSource::BrowserExtension,
            event_seq,
            payload,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_dom_event_translation() {
        let dom_event = BrowserDomEvent {
            tab_id: 101,
            url: "https://example.com/checkout".to_string(),
            page_title: "Checkout Page".to_string(),
            event_type: "CLICK".to_string(),
            tag: "button".to_string(),
            role: Some("button".to_string()),
            visible_text: Some("Pay Now".to_string()),
            aria_label: Some("Pay Now Button".to_string()),
            element_id: Some("btn-pay".to_string()),
            class_name: Some("btn btn-primary".to_string()),
            href: None,
            placeholder: None,
            input_type: None,
            value: None,
            css_selector: Some("#btn-pay".to_string()),
            xpath: Some("//button[@id='btn-pay']".to_string()),
            timestamp_ms: 1700000000,
            is_password: false,
            mutation_info: None,
        };

        let target = dom_event.to_target_metadata();
        assert_eq!(target.name.as_deref(), Some("Pay Now"));
        assert_eq!(target.control_type.as_deref(), Some("button"));
        assert_eq!(target.automation_id.as_deref(), Some("btn-pay"));
        assert_eq!(target.framework_id.as_deref(), Some("DOM"));

        let selector = target.dom_selector.expect("DOM selector present");
        assert_eq!(selector.css_selector.as_deref(), Some("#btn-pay"));
        assert_eq!(selector.xpath.as_deref(), Some("//button[@id='btn-pay']"));
    }

    #[test]
    fn browser_form_values_are_never_exposed_in_target_metadata() {
        let dom_event = BrowserDomEvent {
            tab_id: 101,
            url: "https://example.com/profile".to_string(),
            page_title: "Profile".to_string(),
            event_type: "CHANGE".to_string(),
            tag: "input".to_string(),
            role: Some("textbox".to_string()),
            visible_text: None,
            aria_label: Some("Display name".to_string()),
            element_id: Some("display-name".to_string()),
            class_name: None,
            href: None,
            placeholder: None,
            input_type: Some("text".to_string()),
            value: Some("private user input".to_string()),
            css_selector: Some("#display-name".to_string()),
            xpath: Some("//input[@id='display-name']".to_string()),
            timestamp_ms: 1_700_000_000,
            is_password: false,
            mutation_info: None,
        };

        let target = dom_event.to_target_metadata();
        assert_eq!(target.value.as_deref(), Some("[UNOBSERVED_TEXT]"));
    }

    #[test]
    fn browser_events_can_be_serialized_with_an_unassigned_global_id() {
        let dom_event = BrowserDomEvent {
            tab_id: 7,
            url: "https://example.com/".to_string(),
            page_title: "Example".to_string(),
            event_type: "TAB_CREATED".to_string(),
            tag: "tab".to_string(),
            role: None,
            visible_text: None,
            aria_label: None,
            element_id: None,
            class_name: None,
            href: None,
            placeholder: None,
            input_type: None,
            value: None,
            css_selector: None,
            xpath: None,
            timestamp_ms: 1_700_000_000,
            is_password: false,
            mutation_info: None,
        };

        let raw = dom_event.to_unassigned_raw_event(42, "machine", 1, "user");
        assert_eq!(raw.event_id, 42);
        assert_eq!(raw.source_sequence, 42);
        assert_eq!(raw.global_event_id, Some(GlobalEventId::new(0)));
    }
}
