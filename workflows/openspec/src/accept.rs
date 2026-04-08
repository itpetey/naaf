//! Accept step transformer for workflow systems.
//!
//! This module provides acceptance validation for plans.
//!
//! # Artifact Flow
//! - Reads from: `plan` (Plan from PlanStep)
//! - Writes to: `acceptance` (Acceptance struct with approval status)
//!
//! # Example
//!
//! ```ignore
//! use naaf_openspec::AcceptStep;
//! use naaf_core::steps::Transformer;
//!
//! let accept_step = AcceptStep::new();
//! // Transform state with "plan" artifact to get "acceptance" artifact
//! ```

use std::marker::PhantomData;

use naaf_core::budget::{ExecCtx, Services};
use naaf_core::errors::StepError;
use naaf_core::steps::Transformer;
use naaf_schema::adapters::{AdapterError, IntoState, TryFromState, get_typed, put_typed};
use naaf_schema::artifacts::ArtifactKey;
use naaf_schema::state::StateEnvelope;
use serde::{Deserialize, Serialize};

use crate::llm_json::call_json;
use crate::plan::Plan;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Acceptance {
    pub accepted: bool,
    pub reason: String,
    pub plan_summary: String,
}

impl TryFromState for Acceptance {
    fn try_from_state(key: &ArtifactKey, state: &StateEnvelope) -> Result<Self, AdapterError> {
        let json: serde_json::Value = serde_json::Value::try_from_state(key, state)?;
        serde_json::from_value(json.clone()).map_err(|e| AdapterError::JsonError {
            key: key.to_string(),
            error: e.to_string(),
        })
    }
}

impl IntoState for Acceptance {
    fn into_state(self, key: ArtifactKey, state: &mut StateEnvelope) {
        let json = serde_json::to_value(&self).unwrap();
        json.into_state(key, state);
    }
}

pub struct AcceptStep<S: Services> {
    input_key: ArtifactKey,
    output_key: ArtifactKey,
    _phantom: PhantomData<S>,
}

impl<S: Services> AcceptStep<S> {
    pub fn new() -> Self {
        Self {
            input_key: ArtifactKey::new("plan"),
            output_key: ArtifactKey::new("acceptance"),
            _phantom: PhantomData,
        }
    }

    pub fn with_keys(input_key: impl Into<String>, output_key: impl Into<String>) -> Self {
        Self {
            input_key: ArtifactKey::new(input_key),
            output_key: ArtifactKey::new(output_key),
            _phantom: PhantomData,
        }
    }
}

impl<S: Services> Default for AcceptStep<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Services> Transformer for AcceptStep<S> {
    type Services = S;

    fn name(&self) -> &'static str {
        "accept"
    }

    fn transform(
        &self,
        ctx: &mut ExecCtx<Self::Services>,
        mut state: StateEnvelope,
    ) -> Result<StateEnvelope, StepError> {
        let plan: Plan = get_typed(&self.input_key, &state).map_err(|e| {
            StepError::transformer(
                "accept",
                format!(
                    "Failed to get plan from artifact key '{}': {}",
                    self.input_key, e
                ),
            )
        })?;

        let acceptance: Acceptance = call_json(
            ctx,
            self.name(),
            format!(
                "Return JSON only with keys 'accepted', 'reason', and 'plan_summary'. Review this plan: {}",
                serde_json::to_string(&plan).unwrap_or_default()
            ),
        )?;

        put_typed(self.output_key.clone(), acceptance, &mut state);

        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::EffortLevel;
    use crate::test_services::{JsonSequenceServices, NoopServices};
    use naaf_schema::artifacts::ArtifactValue;
    use naaf_schema::execution_status::ExecutionStatus;
    use naaf_schema::lineage::Lineage;
    use naaf_schema::state::{RunId, StateEnvelope, StateId};
    use naaf_schema::state_kind::StateKind;

    fn make_state_with_plan(effort: EffortLevel) -> StateEnvelope {
        let plan = Plan {
            steps: vec!["Step 1".to_string(), "Step 2".to_string()],
            estimated_effort: effort,
            dependencies: vec![],
        };
        let mut state = StateEnvelope::new(
            StateId::new(),
            RunId::new(),
            StateKind::Proposed,
            Lineage::new(None, None, ExecutionStatus::Pending),
        );
        state.artifacts.insert(
            ArtifactKey::new("plan"),
            ArtifactValue::json(serde_json::json!(plan)),
        );
        state
    }

    fn make_ctx(response: &'static str) -> ExecCtx<JsonSequenceServices> {
        ExecCtx::new(RunId::new(), JsonSequenceServices::from_json([response]))
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_accept_validates_plan() {
        let accept = AcceptStep::new();
        let mut ctx = make_ctx(
            r#"{"accepted":true,"reason":"Plan is straightforward and can proceed immediately","plan_summary":"2 steps: Step 1 → Step 2"}"#,
        );
        let state = make_state_with_plan(EffortLevel::Trivial);

        let result = accept.transform(&mut ctx, state).unwrap();
        let acceptance: Acceptance = get_typed(&ArtifactKey::new("acceptance"), &result).unwrap();
        assert!(acceptance.accepted);
        assert!(acceptance.reason.contains("straightforward"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_accept_moderate_effort() {
        let accept = AcceptStep::new();
        let mut ctx = make_ctx(
            r#"{"accepted":true,"reason":"Plan is clear and reasonable to execute","plan_summary":"2 steps: Step 1 → Step 2"}"#,
        );
        let state = make_state_with_plan(EffortLevel::Moderate);

        let result = accept.transform(&mut ctx, state).unwrap();
        let acceptance: Acceptance = get_typed(&ArtifactKey::new("acceptance"), &result).unwrap();
        assert!(acceptance.accepted);
        assert!(acceptance.reason.contains("clear"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_accept_significant_effort() {
        let accept = AcceptStep::new();
        let mut ctx = make_ctx(
            r#"{"accepted":true,"reason":"Plan requires significant effort but is achievable","plan_summary":"2 steps: Step 1 → Step 2"}"#,
        );
        let state = make_state_with_plan(EffortLevel::Significant);

        let result = accept.transform(&mut ctx, state).unwrap();
        let acceptance: Acceptance = get_typed(&ArtifactKey::new("acceptance"), &result).unwrap();
        assert!(acceptance.accepted);
        assert!(acceptance.reason.contains("significant effort"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_accept_custom_keys() {
        let accept = AcceptStep::with_keys("myplan", "result");
        let plan = Plan {
            steps: vec!["Test".to_string()],
            estimated_effort: EffortLevel::Trivial,
            dependencies: vec![],
        };
        let mut state = StateEnvelope::new(
            StateId::new(),
            RunId::new(),
            StateKind::Proposed,
            Lineage::new(None, None, ExecutionStatus::Pending),
        );
        state.artifacts.insert(
            ArtifactKey::new("myplan"),
            ArtifactValue::json(serde_json::json!(plan)),
        );

        let mut ctx = make_ctx(
            r#"{"accepted":true,"reason":"Plan is straightforward and can proceed immediately","plan_summary":"1 steps: Test"}"#,
        );
        let result = accept.transform(&mut ctx, state).unwrap();

        let acceptance: Acceptance = get_typed(&ArtifactKey::new("result"), &result).unwrap();
        assert!(acceptance.accepted);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_acceptance_includes_plan_summary() {
        let accept = AcceptStep::new();
        let mut ctx = make_ctx(
            r#"{"accepted":true,"reason":"Plan is straightforward and can proceed immediately","plan_summary":"2 steps: Step 1 → Step 2"}"#,
        );
        let state = make_state_with_plan(EffortLevel::Trivial);

        let result = accept.transform(&mut ctx, state).unwrap();
        let acceptance: Acceptance = get_typed(&ArtifactKey::new("acceptance"), &result).unwrap();
        assert!(acceptance.plan_summary.contains("2 steps"));
    }

    #[test]
    fn test_accept_missing_runtime_fails() {
        let accept = AcceptStep::new();
        let mut ctx = ExecCtx::new(RunId::new(), NoopServices);
        let state = make_state_with_plan(EffortLevel::Trivial);

        let result = accept.transform(&mut ctx, state);
        assert!(result.is_err());
    }
}
