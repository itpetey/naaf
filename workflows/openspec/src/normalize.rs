//! Normalize step transformer for workflow systems.
//!
//! This module provides normalization of user input into a structured spec.
//!
//! # Artifact Flow
//! - Reads from: `proposal` (Proposal artifact from ProposeStep)
//! - Writes to: `normalized` (NormalizedInput artifact)

use naaf_core::budget::{DummyServices, ExecCtx};
use naaf_core::errors::StepError;
use naaf_core::steps::Transformer;
use naaf_schema::adapters::{AdapterError, IntoState, TryFromState, get_typed, put_typed};
use naaf_schema::artifacts::ArtifactKey;
use naaf_schema::state::StateEnvelope;
use serde::{Deserialize, Serialize};

use crate::propose::Proposal;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NormalizedInput {
    pub original: String,
    pub normalized: String,
}

impl TryFromState for NormalizedInput {
    fn try_from_state(key: &ArtifactKey, state: &StateEnvelope) -> Result<Self, AdapterError> {
        let json: serde_json::Value = serde_json::Value::try_from_state(key, state)?;
        serde_json::from_value(json.clone()).map_err(|e| AdapterError::JsonError {
            key: key.to_string(),
            error: e.to_string(),
        })
    }
}

impl IntoState for NormalizedInput {
    fn into_state(self, key: ArtifactKey, state: &mut StateEnvelope) {
        let json = serde_json::to_value(&self).unwrap();
        json.into_state(key, state);
    }
}

pub struct NormalizeStep {
    input_key: ArtifactKey,
    output_key: ArtifactKey,
}

impl NormalizeStep {
    /// Creates a new NormalizeStep that reads from "proposal" and writes to "normalized".
    pub fn new() -> Self {
        Self {
            input_key: ArtifactKey::new("proposal"),
            output_key: ArtifactKey::new("normalized"),
        }
    }

    pub fn with_keys(input_key: impl Into<String>, output_key: impl Into<String>) -> Self {
        Self {
            input_key: ArtifactKey::new(input_key),
            output_key: ArtifactKey::new(output_key),
        }
    }

    fn normalize(input: &str) -> String {
        let trimmed = input.trim();
        trimmed.to_lowercase()
    }
}

impl Default for NormalizeStep {
    fn default() -> Self {
        Self::new()
    }
}

impl Transformer for NormalizeStep {
    type Services = DummyServices;

    fn name(&self) -> &'static str {
        "normalize"
    }

    fn transform(
        &self,
        _ctx: &mut ExecCtx<Self::Services>,
        mut state: StateEnvelope,
    ) -> Result<StateEnvelope, StepError> {
        let proposal: Proposal = get_typed(&self.input_key, &state).map_err(|e| {
            StepError::transformer(
                "normalize",
                format!(
                    "Failed to get proposal from artifact key '{}': {}",
                    self.input_key, e
                ),
            )
        })?;

        let normalized = Self::normalize(&proposal.input);
        let normalized_input = NormalizedInput {
            original: proposal.input.clone(),
            normalized,
        };

        put_typed(self.output_key.clone(), normalized_input, &mut state);

        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use naaf_core::budget::DummyServices;
    use naaf_schema::artifacts::ArtifactValue;
    use naaf_schema::execution_status::ExecutionStatus;
    use naaf_schema::lineage::Lineage;
    use naaf_schema::state::{RunId, StateEnvelope, StateId};
    use naaf_schema::state_kind::StateKind;

    fn make_state_with_proposal(input: &str) -> StateEnvelope {
        let mut state = StateEnvelope::new(
            StateId::new(),
            RunId::new(),
            StateKind::Proposed,
            Lineage::new(None, None, ExecutionStatus::Pending),
        );
        let proposal = Proposal {
            input: input.to_string(),
        };
        state.artifacts.insert(
            ArtifactKey::new("proposal"),
            ArtifactValue::json(serde_json::json!(proposal)),
        );
        state
    }

    fn make_ctx() -> ExecCtx<DummyServices> {
        ExecCtx::new(RunId::new(), DummyServices)
    }

    #[test]
    fn test_normalize_creates_normalized_input() {
        let normalize = NormalizeStep::new();
        let mut ctx = make_ctx();
        let state = make_state_with_proposal("  CREATE A File  ");

        let result = normalize.transform(&mut ctx, state).unwrap();
        let normalized: NormalizedInput =
            get_typed(&ArtifactKey::new("normalized"), &result).unwrap();
        assert_eq!(normalized.original, "  CREATE A File  ");
        assert_eq!(normalized.normalized, "create a file");
    }

    #[test]
    fn test_normalize_custom_keys() {
        let normalize = NormalizeStep::with_keys("my_proposal", "result");
        let mut state = StateEnvelope::new(
            StateId::new(),
            RunId::new(),
            StateKind::Proposed,
            Lineage::new(None, None, ExecutionStatus::Pending),
        );
        let proposal = Proposal {
            input: "Hello World".to_string(),
        };
        state.artifacts.insert(
            ArtifactKey::new("my_proposal"),
            ArtifactValue::json(serde_json::json!(proposal)),
        );

        let mut ctx = make_ctx();
        let result = normalize.transform(&mut ctx, state).unwrap();

        let normalized: NormalizedInput = get_typed(&ArtifactKey::new("result"), &result).unwrap();
        assert_eq!(normalized.original, "Hello World");
        assert_eq!(normalized.normalized, "hello world");
    }

    #[test]
    fn test_normalize_missing_proposal() {
        let normalize = NormalizeStep::new();
        let mut ctx = make_ctx();
        let state = StateEnvelope::new(
            StateId::new(),
            RunId::new(),
            StateKind::Proposed,
            Lineage::new(None, None, ExecutionStatus::Pending),
        );

        let result = normalize.transform(&mut ctx, state);
        assert!(result.is_err());
    }
}
