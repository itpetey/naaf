//! Propose step transformer for workflow systems.
//!
//! This module provides the initial proposal transformer that creates
//! a proposal artifact from user input. It is typically the first step
//! in a draft request workflow pipeline.
//!
//! # Artifact Flow
//! - Reads from: `input` (raw user input as String)
//! - Writes to: `proposal` (Proposal struct containing the input)
//!
//! # Example
//!
//! ```ignore
//! use workflow_builtins::ProposeStep;
//! use workflow_core::steps::Transformer;
//!
//! let propose = ProposeStep::new();
//! // Transform state with "input" artifact to get "proposal" artifact
//! ```

use serde::{Deserialize, Serialize};
use workflow_core::budget::{DummyServices, ExecCtx};
use workflow_core::errors::StepError;
use workflow_core::steps::Transformer;
use workflow_schema::adapters::{AdapterError, IntoState, TryFromState, get_typed, put_typed};
use workflow_schema::artifacts::ArtifactKey;
use workflow_schema::state::StateEnvelope;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Proposal {
    pub input: String,
}

impl TryFromState for Proposal {
    fn try_from_state(key: &ArtifactKey, state: &StateEnvelope) -> Result<Self, AdapterError> {
        let json: serde_json::Value = serde_json::Value::try_from_state(key, state)?;
        serde_json::from_value(json.clone()).map_err(|e| AdapterError::JsonError {
            key: key.to_string(),
            error: e.to_string(),
        })
    }
}

impl IntoState for Proposal {
    fn into_state(self, key: ArtifactKey, state: &mut StateEnvelope) {
        let json = serde_json::to_value(&self).unwrap();
        json.into_state(key, state);
    }
}

pub struct ProposeStep {
    input_key: ArtifactKey,
    output_key: ArtifactKey,
}

impl ProposeStep {
    pub fn new() -> Self {
        Self {
            input_key: ArtifactKey::new("input"),
            output_key: ArtifactKey::new("proposal"),
        }
    }

    pub fn with_keys(input_key: impl Into<String>, output_key: impl Into<String>) -> Self {
        Self {
            input_key: ArtifactKey::new(input_key),
            output_key: ArtifactKey::new(output_key),
        }
    }
}

impl Default for ProposeStep {
    fn default() -> Self {
        Self::new()
    }
}

impl Transformer for ProposeStep {
    type Services = DummyServices;

    fn name(&self) -> &'static str {
        "propose"
    }

    fn transform(
        &self,
        _ctx: &mut ExecCtx<Self::Services>,
        mut state: StateEnvelope,
    ) -> Result<StateEnvelope, StepError> {
        let input: String = get_typed(&self.input_key, &state).map_err(|e| {
            StepError::transformer(
                "propose",
                format!(
                    "Failed to get input from artifact key '{}': {}",
                    self.input_key, e
                ),
            )
        })?;

        let proposal = Proposal {
            input: input.clone(),
        };

        put_typed(self.output_key.clone(), proposal, &mut state);

        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use workflow_core::budget::DummyServices;
    use workflow_schema::artifacts::ArtifactValue;
    use workflow_schema::execution_status::ExecutionStatus;
    use workflow_schema::lineage::Lineage;
    use workflow_schema::state::{RunId, StateEnvelope, StateId};
    use workflow_schema::state_kind::StateKind;

    fn make_state_with_input(input: &str) -> StateEnvelope {
        let mut state = StateEnvelope::new(
            StateId::new(),
            RunId::new(),
            StateKind::Proposed,
            Lineage::new(None, None, ExecutionStatus::Pending),
        );
        state
            .artifacts
            .insert(ArtifactKey::new("input"), ArtifactValue::text(input));
        state
    }

    fn make_ctx() -> ExecCtx<DummyServices> {
        ExecCtx::new(RunId::new(), DummyServices)
    }

    #[test]
    fn test_propose_creates_proposal() {
        let propose = ProposeStep::new();
        let mut ctx = make_ctx();
        let state = make_state_with_input("Create a file");

        let result = propose.transform(&mut ctx, state).unwrap();
        let proposal: Proposal = get_typed(&ArtifactKey::new("proposal"), &result).unwrap();
        assert_eq!(proposal.input, "Create a file");
    }

    #[test]
    fn test_propose_custom_keys() {
        let propose = ProposeStep::with_keys("text", "result");
        let mut state = StateEnvelope::new(
            StateId::new(),
            RunId::new(),
            StateKind::Proposed,
            Lineage::new(None, None, ExecutionStatus::Pending),
        );
        state
            .artifacts
            .insert(ArtifactKey::new("text"), ArtifactValue::text("Hello"));

        let mut ctx = make_ctx();
        let result = propose.transform(&mut ctx, state).unwrap();

        let proposal: Proposal = get_typed(&ArtifactKey::new("result"), &result).unwrap();
        assert_eq!(proposal.input, "Hello");
    }

    #[test]
    fn test_propose_missing_input() {
        let propose = ProposeStep::new();
        let mut ctx = make_ctx();
        let state = StateEnvelope::new(
            StateId::new(),
            RunId::new(),
            StateKind::Proposed,
            Lineage::new(None, None, ExecutionStatus::Pending),
        );

        let result = propose.transform(&mut ctx, state);
        assert!(result.is_err());
    }
}
