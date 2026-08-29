use harness_app::{HarnessEventLog, HarnessFixture, ScriptedAction};
use std::path::Path;

pub struct HarnessClient {
    fixture: HarnessFixture,
}

impl Default for HarnessClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HarnessClient {
    pub fn new() -> Self {
        Self {
            fixture: HarnessFixture::new(),
        }
    }

    pub fn execute_action(&mut self, action: &ScriptedAction) -> Result<(), String> {
        self.fixture.execute_action(action)
    }

    pub fn execute_script(&mut self, script: &[ScriptedAction]) -> Result<(), String> {
        self.fixture.execute_script(script)
    }

    pub fn get_event_logs(&self) -> Vec<HarnessEventLog> {
        self.fixture.state.event_logs.clone()
    }

    pub fn execute_script_file(
        &mut self,
        script_path: &Path,
    ) -> Result<Vec<HarnessEventLog>, String> {
        let content = std::fs::read_to_string(script_path)
            .map_err(|e| format!("Failed to read script {}: {}", script_path.display(), e))?;
        let actions: Vec<ScriptedAction> = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse script JSON: {}", e))?;
        self.execute_script(&actions)?;
        Ok(self.get_event_logs())
    }
}
