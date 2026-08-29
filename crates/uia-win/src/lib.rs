//! UI Automation COM engine and element inspector with 3-level tree walking.

pub mod focus;
pub mod inspector;
pub mod mock;
pub mod model;
pub mod walker;

pub use focus::{FocusChange, FocusChangeDetector, FocusEvidence};
pub use inspector::UiaInspector;
pub use mock::{MockUiaStore, create_mock_button, create_mock_password_box};
pub use model::{UiaAncestorInfo, UiaElementInfo, control_type_id_to_name};

#[cfg(test)]
mod tests {
    use super::*;
    use core_types::metadata::BoundingRect;

    #[tokio::test]
    async fn test_mock_uia_inspection_and_ancestors() {
        let store = std::sync::Arc::new(MockUiaStore::new());
        let button = create_mock_button("SubmitBtn", "btn_submit", 100, 200, 80, 30);
        store.add_element(button);

        let inspector = UiaInspector::with_mock_store(store);

        // Point inside button
        let target = inspector
            .inspect_point(120, 210)
            .await
            .expect("Element found");
        assert_eq!(target.name.as_deref(), Some("SubmitBtn"));
        assert_eq!(target.control_type.as_deref(), Some("Button"));
        assert_eq!(target.automation_id.as_deref(), Some("btn_submit"));
        assert_eq!(target.framework_id.as_deref(), Some("WPF"));
        assert_eq!(target.is_password, false);

        // Verify 3-level ancestor chain
        assert_eq!(target.ancestor_chain.len(), 3);
        assert_eq!(target.ancestor_chain[0].level, 1);
        assert_eq!(
            target.ancestor_chain[0].control_type.as_deref(),
            Some("ToolBar")
        );
        assert_eq!(target.ancestor_chain[1].level, 2);
        assert_eq!(
            target.ancestor_chain[1].control_type.as_deref(),
            Some("Group")
        );
        assert_eq!(target.ancestor_chain[2].level, 3);
        assert_eq!(
            target.ancestor_chain[2].control_type.as_deref(),
            Some("Window")
        );

        // Point outside button
        let outside = inspector.inspect_point(50, 50).await;
        assert!(outside.is_none());
    }

    #[tokio::test]
    async fn test_password_element_redaction() {
        let store = std::sync::Arc::new(MockUiaStore::new());
        let pass_box = create_mock_password_box("Password", "txt_pass", 100, 100, 200, 25);
        store.add_element(pass_box);

        let inspector = UiaInspector::with_mock_store(store);
        let target = inspector
            .inspect_point(150, 110)
            .await
            .expect("Element found");
        assert!(target.is_password);
        assert_eq!(target.value.as_deref(), Some("[PASSWORD_REDACTED]"));
    }

    #[test]
    fn test_control_type_mapping() {
        assert_eq!(control_type_id_to_name(50000), "Button");
        assert_eq!(control_type_id_to_name(50004), "Edit");
        assert_eq!(control_type_id_to_name(50032), "Window");
        assert_eq!(control_type_id_to_name(99999), "Unknown");
    }

    #[tokio::test]
    async fn focus_detector_emits_only_transitions_and_never_exposes_values() {
        let store = std::sync::Arc::new(MockUiaStore::new());
        let mut first = create_mock_button("Save", "save_button", 10, 20, 100, 30);
        first.value = Some("must not be persisted".to_string());
        store.set_focused(Some(first));

        let inspector = UiaInspector::with_mock_store(store.clone());
        let mut detector = FocusChangeDetector::new();

        let gained = inspector
            .inspect_focus_change(&mut detector)
            .await
            .expect("initial focus must be evidence");
        assert_eq!(gained.change, FocusChange::Gained);
        assert_eq!(
            gained
                .target
                .as_ref()
                .and_then(|target| target.value.as_deref()),
            None
        );

        assert!(
            inspector
                .inspect_focus_change(&mut detector)
                .await
                .is_none()
        );

        store.set_focused(Some(create_mock_button(
            "Cancel",
            "cancel_button",
            10,
            20,
            100,
            30,
        )));
        let changed = inspector
            .inspect_focus_change(&mut detector)
            .await
            .expect("a different focused control must be evidence");
        assert_eq!(changed.change, FocusChange::Changed);
        assert_eq!(
            changed
                .previous
                .as_ref()
                .and_then(|target| target.automation_id.as_deref()),
            Some("save_button")
        );
        assert_eq!(
            changed
                .target
                .as_ref()
                .and_then(|target| target.automation_id.as_deref()),
            Some("cancel_button")
        );

        store.set_focused(None);
        let lost = inspector
            .inspect_focus_change(&mut detector)
            .await
            .expect("loss of focus must be evidence");
        assert_eq!(lost.change, FocusChange::Lost);
        assert!(lost.target.is_none());
    }

    #[test]
    fn focus_detector_bounds_identity_metadata() {
        let mut detector = FocusChangeDetector::new();
        let long_name = "x".repeat(600);
        let evidence = detector
            .observe(Some(
                UiaElementInfo {
                    name: Some(long_name),
                    control_type: "Edit".to_string(),
                    automation_id: Some("field".to_string()),
                    ..Default::default()
                }
                .to_target_metadata(),
            ))
            .expect("initial focus must emit evidence");

        assert_eq!(
            evidence
                .target
                .expect("target")
                .name
                .expect("name")
                .chars()
                .count(),
            256
        );
    }
}
