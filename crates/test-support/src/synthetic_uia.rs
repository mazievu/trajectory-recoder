use core_types::{BoundingBox, TargetMetadata};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct SyntheticUiaElement {
    pub id: String,
    pub name: String,
    pub control_type: String,
    pub class_name: String,
    pub automation_id: String,
    pub framework_id: String,
    pub bounds: BoundingBox,
    pub is_password: bool,
    pub value: Option<String>,
    pub children: Vec<SyntheticUiaElement>,
}

impl SyntheticUiaElement {
    pub fn contains_point(&self, x: i32, y: i32) -> bool {
        self.bounds.contains(x, y)
    }

    pub fn to_target_metadata(&self) -> TargetMetadata {
        TargetMetadata {
            name: Some(self.name.clone()),
            control_type: Some(self.control_type.clone()),
            automation_id: Some(self.automation_id.clone()),
            class_name: Some(self.class_name.clone()),
            framework_id: Some(self.framework_id.clone()),
            bounding_box: Some(self.bounds),
            bounding_rect: Some(self.bounds.to_bounding_rect()),
            is_password: self.is_password,
            is_enabled: Some(true),
            is_keyboard_focusable: Some(true),
            value: self.value.clone(),
            help_text: None,
            ancestor_chain: vec![],
            ancestors: vec![],
            dom_selector: None,
            xpath: None,
        }
    }
}

pub struct SyntheticUiaTree {
    pub root: SyntheticUiaElement,
    pub simulated_latency: Duration,
    pub simulate_hang: bool,
}

impl SyntheticUiaTree {
    pub fn new_standard_form() -> Self {
        let root = SyntheticUiaElement {
            id: "window_1".to_string(),
            name: "Customer Entry Form".to_string(),
            control_type: "Window".to_string(),
            class_name: "StandardWindow".to_string(),
            automation_id: "wnd_main".to_string(),
            framework_id: "Win32".to_string(),
            bounds: BoundingBox {
                x: 0,
                y: 0,
                width: 800,
                height: 600,
            },
            is_password: false,
            value: None,
            children: vec![
                SyntheticUiaElement {
                    id: "txt_user".to_string(),
                    name: "Username".to_string(),
                    control_type: "Edit".to_string(),
                    class_name: "TextBox".to_string(),
                    automation_id: "txt_username".to_string(),
                    framework_id: "Win32".to_string(),
                    bounds: BoundingBox {
                        x: 50,
                        y: 100,
                        width: 200,
                        height: 30,
                    },
                    is_password: false,
                    value: Some("john_doe".to_string()),
                    children: vec![],
                },
                SyntheticUiaElement {
                    id: "txt_pass".to_string(),
                    name: "Password".to_string(),
                    control_type: "Edit".to_string(),
                    class_name: "PasswordBox".to_string(),
                    automation_id: "txt_password".to_string(),
                    framework_id: "Win32".to_string(),
                    bounds: BoundingBox {
                        x: 50,
                        y: 150,
                        width: 200,
                        height: 30,
                    },
                    is_password: true,
                    value: Some("Secret123!".to_string()),
                    children: vec![],
                },
                SyntheticUiaElement {
                    id: "btn_sub".to_string(),
                    name: "Submit".to_string(),
                    control_type: "Button".to_string(),
                    class_name: "Button".to_string(),
                    automation_id: "btn_submit".to_string(),
                    framework_id: "Win32".to_string(),
                    bounds: BoundingBox {
                        x: 50,
                        y: 200,
                        width: 100,
                        height: 35,
                    },
                    is_password: false,
                    value: None,
                    children: vec![],
                },
            ],
        };

        Self {
            root,
            simulated_latency: Duration::from_millis(1),
            simulate_hang: false,
        }
    }

    pub async fn query_element_at_point(
        &self,
        x: i32,
        y: i32,
        timeout: Duration,
    ) -> Result<Option<TargetMetadata>, String> {
        if self.simulate_hang {
            tokio::time::sleep(timeout + Duration::from_millis(50)).await;
            return Err("UIA COM call timed out".to_string());
        }

        tokio::time::sleep(self.simulated_latency).await;

        fn search_node(
            node: &SyntheticUiaElement,
            x: i32,
            y: i32,
            ancestors: &mut Vec<TargetMetadata>,
        ) -> Option<TargetMetadata> {
            if !node.contains_point(x, y) {
                return None;
            }
            // Check children first for deepest match
            for child in &node.children {
                ancestors.push(node.to_target_metadata());
                if let Some(res) = search_node(child, x, y, ancestors) {
                    return Some(res);
                }
                ancestors.pop();
            }
            let mut target = node.to_target_metadata();
            target.ancestors = ancestors.clone();
            Some(target)
        }

        let mut ancestors = Vec::new();
        Ok(search_node(&self.root, x, y, &mut ancestors))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_synthetic_uia_tree_hit_test() {
        let tree = SyntheticUiaTree::new_standard_form();
        let target = tree
            .query_element_at_point(100, 110, Duration::from_millis(100))
            .await
            .unwrap();
        assert!(target.is_some());
        let meta = target.unwrap();
        assert_eq!(meta.automation_id, Some("txt_username".to_string()));
        assert_eq!(meta.ancestors.len(), 1);
        assert_eq!(
            meta.ancestors[0].name,
            Some("Customer Entry Form".to_string())
        );
    }

    #[tokio::test]
    async fn test_synthetic_uia_hang_timeout_simulation() {
        let mut tree = SyntheticUiaTree::new_standard_form();
        tree.simulate_hang = true;
        let res = tree
            .query_element_at_point(100, 110, Duration::from_millis(10))
            .await;
        assert!(res.is_err());
    }
}
