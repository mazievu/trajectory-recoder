use crate::controls::*;
use crate::uia_provider::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessEventLog {
    pub timestamp: DateTime<Utc>,
    pub control_id: String,
    pub event_type: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptedAction {
    pub step_id: String,
    pub action: String, // "click", "type", "clear", "scroll", "drag_drop", "open_dialog", "close_dialog", "wait"
    pub target_id: String,
    pub param_str: Option<String>,
    pub param_f64_a: Option<f64>,
    pub param_f64_b: Option<f64>,
    pub delay_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessState {
    pub window_title: String,
    pub is_active: bool,
    pub buttons: HashMap<String, ButtonControl>,
    pub inputs: HashMap<String, InputControl>,
    pub scrolls: HashMap<String, ScrollControl>,
    pub drag_drop: DragDropZone,
    pub dialogs: HashMap<String, DialogControl>,
    pub progress: ProgressControl,
    pub event_logs: Vec<HarnessEventLog>,
}

pub struct HarnessFixture {
    pub state: HarnessState,
    pub uia_root: UiaElementNode,
}

impl Default for HarnessFixture {
    fn default() -> Self {
        Self::new()
    }
}

impl HarnessFixture {
    pub fn new() -> Self {
        let mut buttons = HashMap::new();
        buttons.insert(
            "btn_submit".to_string(),
            ButtonControl::new("btn_submit", "Submit", ButtonKind::Submit),
        );
        buttons.insert(
            "btn_toggle".to_string(),
            ButtonControl::new("btn_toggle", "Toggle Mode", ButtonKind::Toggle),
        );
        buttons.insert(
            "btn_cancel".to_string(),
            ButtonControl::new("btn_cancel", "Cancel", ButtonKind::Push),
        );

        let mut inputs = HashMap::new();
        inputs.insert(
            "txt_username".to_string(),
            InputControl::new("txt_username", "Username", InputKind::SingleLine),
        );
        inputs.insert(
            "txt_password".to_string(),
            InputControl::new("txt_password", "Password", InputKind::Password),
        );
        inputs.insert(
            "txt_credit_card".to_string(),
            InputControl::new("txt_credit_card", "Credit Card", InputKind::CreditCard),
        );
        inputs.insert(
            "txt_notes".to_string(),
            InputControl::new("txt_notes", "Notes", InputKind::MultiLine),
        );

        let mut scrolls = HashMap::new();
        scrolls.insert(
            "pnl_scrollable".to_string(),
            ScrollControl::new(
                "pnl_scrollable",
                "Data Grid Viewport",
                800.0,
                600.0,
                800.0,
                5000.0,
            ),
        );

        let drag_drop = DragDropZone::new("drag_source_item", "drop_target_zone");

        let mut dialogs = HashMap::new();
        dialogs.insert(
            "dlg_open_file".to_string(),
            DialogControl::new("dlg_open_file", "Open Document", DialogKind::OpenFile),
        );
        dialogs.insert(
            "dlg_confirm_save".to_string(),
            DialogControl::new("dlg_confirm_save", "Confirm Save", DialogKind::Information),
        );

        let progress = ProgressControl::new("spinner_async_op", "Operation Progress");

        let mut uia_root = UiaElementNode::new(
            "wnd_harness_main",
            "Trajectory Test Harness Application",
            "HarnessWindow",
            UiaControlType::Window,
            UiaBoundingRect {
                left: 100.0,
                top: 100.0,
                width: 1024.0,
                height: 768.0,
            },
        );

        // Populate UIA children
        uia_root.children.push(UiaElementNode::new(
            "btn_submit",
            "Submit",
            "Button",
            UiaControlType::Button,
            UiaBoundingRect {
                left: 120.0,
                top: 150.0,
                width: 100.0,
                height: 35.0,
            },
        ));
        uia_root.children.push(UiaElementNode::new(
            "btn_toggle",
            "Toggle Mode",
            "Button",
            UiaControlType::Button,
            UiaBoundingRect {
                left: 230.0,
                top: 150.0,
                width: 120.0,
                height: 35.0,
            },
        ));
        uia_root.children.push(UiaElementNode::new(
            "txt_username",
            "Username",
            "Edit",
            UiaControlType::Edit,
            UiaBoundingRect {
                left: 120.0,
                top: 200.0,
                width: 250.0,
                height: 30.0,
            },
        ));

        let mut pwd_node = UiaElementNode::new(
            "txt_password",
            "Password",
            "Edit",
            UiaControlType::Edit,
            UiaBoundingRect {
                left: 120.0,
                top: 250.0,
                width: 250.0,
                height: 30.0,
            },
        );
        pwd_node.is_password = true;
        uia_root.children.push(pwd_node);

        uia_root.children.push(UiaElementNode::new(
            "pnl_scrollable",
            "Data Grid Viewport",
            "Pane",
            UiaControlType::Pane,
            UiaBoundingRect {
                left: 400.0,
                top: 150.0,
                width: 500.0,
                height: 400.0,
            },
        ));

        Self {
            state: HarnessState {
                window_title: "Trajectory Test Harness Application".to_string(),
                is_active: true,
                buttons,
                inputs,
                scrolls,
                drag_drop,
                dialogs,
                progress,
                event_logs: Vec::new(),
            },
            uia_root,
        }
    }

    pub fn execute_action(&mut self, action: &ScriptedAction) -> Result<(), String> {
        let ts = Utc::now();
        match action.action.as_str() {
            "click" => {
                if let Some(btn) = self.state.buttons.get_mut(&action.target_id) {
                    btn.click();
                    self.state.event_logs.push(HarnessEventLog {
                        timestamp: ts,
                        control_id: action.target_id.clone(),
                        event_type: "click".to_string(),
                        payload: serde_json::json!({ "click_count": btn.click_count, "is_checked": btn.is_checked }),
                    });
                    Ok(())
                } else {
                    Err(format!("Button control '{}' not found", action.target_id))
                }
            }
            "type" => {
                if let Some(inp) = self.state.inputs.get_mut(&action.target_id) {
                    let text = action.param_str.as_deref().unwrap_or("");
                    inp.append_text(text);
                    self.state.event_logs.push(HarnessEventLog {
                        timestamp: ts,
                        control_id: action.target_id.clone(),
                        event_type: "type".to_string(),
                        payload: serde_json::json!({
                            "is_password": inp.is_password,
                            "len": text.len(),
                        }),
                    });
                    Ok(())
                } else {
                    Err(format!("Input control '{}' not found", action.target_id))
                }
            }
            "scroll" => {
                if let Some(scr) = self.state.scrolls.get_mut(&action.target_id) {
                    let dx = action.param_f64_a.unwrap_or(0.0);
                    let dy = action.param_f64_b.unwrap_or(0.0);
                    scr.scroll(dx, dy);
                    self.state.event_logs.push(HarnessEventLog {
                        timestamp: ts,
                        control_id: action.target_id.clone(),
                        event_type: "scroll".to_string(),
                        payload: serde_json::json!({
                            "offset_x": scr.offset_x,
                            "offset_y": scr.offset_y,
                            "delta_x": dx,
                            "delta_y": dy,
                        }),
                    });
                    Ok(())
                } else {
                    Err(format!("Scroll control '{}' not found", action.target_id))
                }
            }
            "drag_drop" => {
                let payload = action.param_str.as_deref().unwrap_or("item_payload");
                self.state.drag_drop.start_drag(payload);
                let dropped = self.state.drag_drop.complete_drop();
                self.state.event_logs.push(HarnessEventLog {
                    timestamp: ts,
                    control_id: format!(
                        "{}_to_{}",
                        self.state.drag_drop.source_id, self.state.drag_drop.target_id
                    ),
                    event_type: "drag_drop".to_string(),
                    payload: serde_json::json!({ "dropped": dropped }),
                });
                Ok(())
            }
            "open_dialog" => {
                if let Some(dlg) = self.state.dialogs.get_mut(&action.target_id) {
                    let msg = action.param_str.as_deref().unwrap_or("Select file");
                    dlg.open(msg);
                    self.state.event_logs.push(HarnessEventLog {
                        timestamp: ts,
                        control_id: action.target_id.clone(),
                        event_type: "open_dialog".to_string(),
                        payload: serde_json::json!({ "dialog_title": dlg.title }),
                    });
                    Ok(())
                } else {
                    Err(format!("Dialog '{}' not found", action.target_id))
                }
            }
            "close_dialog" => {
                if let Some(dlg) = self.state.dialogs.get_mut(&action.target_id) {
                    let res_str = action.param_str.as_deref().unwrap_or("ok");
                    let result = match res_str {
                        "ok" => DialogResult::Ok,
                        "cancel" => DialogResult::Cancel,
                        path if !path.is_empty() => DialogResult::FileSelected(path.to_string()),
                        _ => DialogResult::None,
                    };
                    dlg.close(result);
                    self.state.event_logs.push(HarnessEventLog {
                        timestamp: ts,
                        control_id: action.target_id.clone(),
                        event_type: "close_dialog".to_string(),
                        payload: serde_json::json!({ "result": res_str }),
                    });
                    Ok(())
                } else {
                    Err(format!("Dialog '{}' not found", action.target_id))
                }
            }
            "start_progress" => {
                let latency = action.delay_ms.unwrap_or(1800);
                self.state.progress.start(latency);
                self.state.event_logs.push(HarnessEventLog {
                    timestamp: ts,
                    control_id: self.state.progress.automation_id.clone(),
                    event_type: "start_progress".to_string(),
                    payload: serde_json::json!({ "latency_ms": latency }),
                });
                Ok(())
            }
            "finish_progress" => {
                self.state.progress.finish();
                self.state.event_logs.push(HarnessEventLog {
                    timestamp: ts,
                    control_id: self.state.progress.automation_id.clone(),
                    event_type: "finish_progress".to_string(),
                    payload: serde_json::json!({ "progress_percent": 100.0 }),
                });
                Ok(())
            }
            "wait" => {
                // Sleep simulation
                let ms = action.delay_ms.unwrap_or(100);
                std::thread::sleep(std::time::Duration::from_millis(ms));
                Ok(())
            }
            other => Err(format!("Unknown scripted action '{}'", other)),
        }
    }

    pub fn execute_script(&mut self, actions: &[ScriptedAction]) -> Result<(), String> {
        for action in actions {
            self.execute_action(action)?;
        }
        Ok(())
    }
}
