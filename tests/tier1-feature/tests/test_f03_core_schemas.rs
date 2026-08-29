use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActionType {
    Click,
    TypeText,
    Shortcut,
    Scroll,
    DragDrop,
    WindowSwitch,
    AppOpen,
    FileOpen,
    FileSave,
    Wait,
    StateChange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalAction {
    pub schema: String,
    pub schema_version: String,
    pub global_event_id: u64,
    pub session_id: String,
    pub action_type: ActionType,
    pub confidence: f32,
    pub target_id: Option<String>,
}

#[test]
fn test_f03_canonical_action_schema_validation() {
    let action = CanonicalAction {
        schema: "gtf.trajectory".to_string(),
        schema_version: "1.0".to_string(),
        global_event_id: 10452,
        session_id: "machine01_20260829_090000_1234".to_string(),
        action_type: ActionType::Click,
        confidence: 0.95,
        target_id: Some("btn_save".to_string()),
    };

    let encoded = serde_json::to_string(&action).unwrap();
    let decoded: CanonicalAction = serde_json::from_str(&encoded).unwrap();

    assert_eq!(decoded.schema, "gtf.trajectory");
    assert_eq!(decoded.schema_version, "1.0");
    assert_eq!(decoded.global_event_id, 10452);
    assert_eq!(decoded.action_type, ActionType::Click);
    assert_eq!(decoded.confidence, 0.95);
    assert_eq!(decoded.target_id, Some("btn_save".to_string()));
}
