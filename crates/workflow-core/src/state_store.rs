//! State persistence for workflow execution.
//!
//! Provides filesystem-based persistence for workflow state snapshots.

use std::path::Path;
use workflow_schema::state::StateEnvelope;

use crate::errors::Error;

pub struct StateStore;

impl StateStore {
    pub fn save(state: &StateEnvelope, dir: &Path) -> Result<(), Error> {
        std::fs::create_dir_all(dir)
            .map_err(|e| Error::Persistence(format!("Failed to create directory: {}", e)))?;

        let state_file = dir.join("state.json");
        let json = serde_json::to_string_pretty(state)
            .map_err(|e| Error::Persistence(format!("Failed to serialize state: {}", e)))?;

        std::fs::write(&state_file, json)
            .map_err(|e| Error::Persistence(format!("Failed to write state: {}", e)))?;

        let artifacts_file = dir.join("artifacts.json");
        let artifacts_json = serde_json::to_string_pretty(&state.artifacts)
            .map_err(|e| Error::Persistence(format!("Failed to serialize artifacts: {}", e)))?;

        std::fs::write(&artifacts_file, artifacts_json)
            .map_err(|e| Error::Persistence(format!("Failed to write artifacts: {}", e)))?;

        Ok(())
    }

    pub fn load(dir: &Path) -> Result<StateEnvelope, Error> {
        let state_file = dir.join("state.json");

        if !state_file.exists() {
            return Err(Error::Persistence(format!(
                "State file not found: {}",
                state_file.display()
            )));
        }

        let json = std::fs::read_to_string(&state_file)
            .map_err(|e| Error::Persistence(format!("Failed to read state: {}", e)))?;

        serde_json::from_str(&json)
            .map_err(|e| Error::Persistence(format!("Failed to deserialize state: {}", e)))
    }
}
