//! UI Automation COM engine and element inspector with 3-level tree walking.

pub mod inspector;
pub mod mock;
pub mod model;
pub mod walker;

pub use inspector::UiaInspector;
pub use mock::{create_mock_button, create_mock_password_box, MockUiaStore};
pub use model::{control_type_id_to_name, UiaAncestorInfo, UiaElementInfo};

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
        let target = inspector.inspect_point(120, 210).await.expect("Element found");
        assert_eq!(target.name.as_deref(), Some("SubmitBtn"));
        assert_eq!(target.control_type.as_deref(), Some("Button"));
        assert_eq!(target.automation_id.as_deref(), Some("btn_submit"));
        assert_eq!(target.framework_id.as_deref(), Some("WPF"));
        assert_eq!(target.is_password, false);

        // Verify 3-level ancestor chain
        assert_eq!(target.ancestor_chain.len(), 3);
        assert_eq!(target.ancestor_chain[0].level, 1);
        assert_eq!(target.ancestor_chain[0].control_type.as_deref(), Some("ToolBar"));
        assert_eq!(target.ancestor_chain[1].level, 2);
        assert_eq!(target.ancestor_chain[1].control_type.as_deref(), Some("Group"));
        assert_eq!(target.ancestor_chain[2].level, 3);
        assert_eq!(target.ancestor_chain[2].control_type.as_deref(), Some("Window"));

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
        let target = inspector.inspect_point(150, 110).await.expect("Element found");
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
}
