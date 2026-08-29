use crate::agent_controller::SpoolDirectoryManager;
use crate::harness_client::HarnessClient;
use crate::mock_server::{start_mock_server, MockServerHandle};
use crate::verifiers::*;
use harness_app::ScriptedAction;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeAuditResult {
    pub attribute_number: usize,
    pub attribute_name: String,
    pub is_verified: bool,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconstructionAuditReport {
    pub total_attributes: usize,
    pub verified_attributes: usize,
    pub failed_attributes: usize,
    pub checklist: Vec<AttributeAuditResult>,
    pub is_100_percent_compliant: bool,
}

pub struct ScenarioRunner {
    pub temp_dir: TempDir,
    pub spool: SpoolDirectoryManager,
    pub harness: HarnessClient,
}

impl Default for ScenarioRunner {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

impl ScenarioRunner {
    pub fn new() -> Result<Self, std::io::Error> {
        let temp_dir = TempDir::new()?;
        let spool = SpoolDirectoryManager::new(temp_dir.path().join("spool"))?;
        let harness = HarnessClient::new();
        Ok(Self {
            temp_dir,
            spool,
            harness,
        })
    }

    pub fn spool_path(&self) -> PathBuf {
        self.spool.base_path.clone()
    }

    pub fn create_test_session(&self, session_id: &str) -> Result<PathBuf, std::io::Error> {
        let s_dir = self.spool.recording_dir().join(session_id);
        fs::create_dir_all(&s_dir)?;
        fs::create_dir_all(s_dir.join("screenshots"))?;
        fs::create_dir_all(s_dir.join("video"))?;

        // Create manifest.json
        let manifest_content = serde_json::json!({
            "schema_version": "1.0",
            "session_id": session_id,
            "machine_id": "test_machine_01",
            "started_at": chrono::Utc::now().to_rfc3339(),
            "status": "recording"
        });
        fs::write(s_dir.join("manifest.json"), serde_json::to_string_pretty(&manifest_content)?)?;

        Ok(s_dir)
    }

    pub fn write_sample_ndjson_events(
        session_dir: &Path,
        events: &[serde_json::Value],
    ) -> Result<(), std::io::Error> {
        let raw_path = session_dir.join("events.raw.ndjson");
        let mut file = File::create(raw_path)?;
        for ev in events {
            let line = serde_json::to_string(ev)?;
            writeln!(file, "{}", line)?;
        }
        file.flush()?;
        Ok(())
    }

    pub fn audit_19_attributes(canonical_events: &[serde_json::Value]) -> ReconstructionAuditReport {
        let attribute_definitions = [
            (1, "Application Launches", "APP_OPEN"),
            (2, "Application Switches", "WINDOW_SWITCH"),
            (3, "Active Window States", "WINDOW_STATE"),
            (4, "Clicked UI Targets", "CLICK"),
            (5, "Typed Text Inputs", "TYPE_TEXT"),
            (6, "Keyboard Shortcuts", "SHORTCUT"),
            (7, "Clipboard Copy Sources", "COPY"),
            (8, "Clipboard Paste Targets", "PASTE"),
            (9, "Opened Files", "FILE_OPEN"),
            (10, "Selected Files", "DIALOG_CONFIRM"),
            (11, "Uploaded Files", "FILE_UPLOAD"),
            (12, "Downloaded Files", "FILE_DOWNLOAD"),
            (13, "Drag & Drop Source/Dest", "DRAG_DROP"),
            (14, "Scroll Containers", "SCROLL"),
            (15, "Appeared Dialogs", "DIALOG_OPEN"),
            (16, "Dialog Confirmations", "DIALOG_ACTION"),
            (17, "System Wait Durations", "WAIT"),
            (18, "Result & State Changes", "STATE_CHANGE"),
            (19, "Final Workflow Output", "TERMINAL_STATE"),
        ];

        let mut checklist = Vec::new();
        let mut verified_count = 0;

        for (num, name, action_type_match) in attribute_definitions {
            let found = canonical_events.iter().any(|ev| {
                ev.get("action_type")
                    .and_then(|a| a.as_str())
                    .map(|s| s == action_type_match)
                    .unwrap_or(false)
            });

            if found {
                verified_count += 1;
                checklist.push(AttributeAuditResult {
                    attribute_number: num,
                    attribute_name: name.to_string(),
                    is_verified: true,
                    details: format!("Verified presence of action_type '{}'", action_type_match),
                });
            } else {
                checklist.push(AttributeAuditResult {
                    attribute_number: num,
                    attribute_name: name.to_string(),
                    is_verified: false,
                    details: format!("Missing evidence for action_type '{}'", action_type_match),
                });
            }
        }

        let total = attribute_definitions.len();
        ReconstructionAuditReport {
            total_attributes: total,
            verified_attributes: verified_count,
            failed_attributes: total - verified_count,
            checklist,
            is_100_percent_compliant: verified_count == total,
        }
    }
}
