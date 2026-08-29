use harness_app::{HarnessFixture, ScriptedAction};
use std::env;
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let mut mode = "headless".to_string();
    let mut script_path: Option<String> = None;
    let mut output_path: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--mode" => {
                if i + 1 < args.len() {
                    mode = args[i + 1].clone();
                    i += 1;
                }
            }
            "--script" => {
                if i + 1 < args.len() {
                    script_path = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--output" => {
                if i + 1 < args.len() {
                    output_path = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--help" | "-h" => {
                println!("Trajectory Test Harness Application (trajectory-harness.exe)");
                println!("Usage: trajectory-harness [OPTIONS]");
                println!("Options:");
                println!("  --mode <headless|interactive>   Execution mode (default: headless)");
                println!("  --script <path.json>            Run a predefined JSON script of actions");
                println!("  --output <path.json>            Save fixture event logs to file");
                println!("  --help, -h                      Show this help message");
                return Ok(());
            }
            _ => {}
        }
        i += 1;
    }

    println!("[trajectory-harness] Starting harness in '{}' mode", mode);
    let mut fixture = HarnessFixture::new();

    if let Some(ref sp) = script_path {
        println!("[trajectory-harness] Loading scenario script from '{}'", sp);
        let content = fs::read_to_string(Path::new(sp))?;
        let actions: Vec<ScriptedAction> = serde_json::from_str(&content)?;
        println!("[trajectory-harness] Executing {} scripted actions...", actions.len());
        fixture.execute_script(&actions)
            .map_err(|e| format!("Script execution failed: {}", e))?;
        println!("[trajectory-harness] Script completed successfully.");
    } else {
        println!("[trajectory-harness] No script provided. Executing default smoke test...");
        let default_actions = vec![
            ScriptedAction {
                step_id: "step_1".to_string(),
                action: "click".to_string(),
                target_id: "btn_toggle".to_string(),
                param_str: None,
                param_f64_a: None,
                param_f64_b: None,
                delay_ms: None,
            },
            ScriptedAction {
                step_id: "step_2".to_string(),
                action: "type".to_string(),
                target_id: "txt_username".to_string(),
                param_str: Some("test_user_01".to_string()),
                param_f64_a: None,
                param_f64_b: None,
                delay_ms: None,
            },
            ScriptedAction {
                step_id: "step_3".to_string(),
                action: "type".to_string(),
                target_id: "txt_password".to_string(),
                param_str: Some("SuperSecretPassword123!".to_string()),
                param_f64_a: None,
                param_f64_b: None,
                delay_ms: None,
            },
            ScriptedAction {
                step_id: "step_4".to_string(),
                action: "scroll".to_string(),
                target_id: "pnl_scrollable".to_string(),
                param_str: None,
                param_f64_a: Some(0.0),
                param_f64_b: Some(120.0),
                delay_ms: None,
            },
            ScriptedAction {
                step_id: "step_5".to_string(),
                action: "click".to_string(),
                target_id: "btn_submit".to_string(),
                param_str: None,
                param_f64_a: None,
                param_f64_b: None,
                delay_ms: None,
            },
        ];
        fixture.execute_script(&default_actions)
            .map_err(|e| format!("Default actions failed: {}", e))?;
        println!("[trajectory-harness] Default smoke test finished.");
    }

    let summary = serde_json::to_string_pretty(&fixture.state.event_logs)?;
    if let Some(ref op) = output_path {
        fs::write(Path::new(op), &summary)?;
        println!("[trajectory-harness] Event log written to '{}'", op);
    } else {
        println!("[trajectory-harness] Event log:\n{}", summary);
    }

    Ok(())
}
