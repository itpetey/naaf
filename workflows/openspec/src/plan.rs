//! Plan step transformer for workflow systems.
//!
//! This module provides planning capability that creates a plan from scope analysis.
//!
//! # Artifact Flow
//! - Reads from: `scope` (ScopeAnalysis from ScopeStep)
//! - Writes to: `plan` (Plan struct with steps and estimated effort)
//!
//! # Example
//!
//! ```ignore
//! use naaf_openspec::PlanStep;
//! use naaf_core::steps::Transformer;
//!
//! let plan_step = PlanStep::new();
//! // Transform state with "scope" artifact to get "plan" artifact
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
use crate::scope::ScopeAnalysis;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Plan {
    pub steps: Vec<String>,
    pub estimated_effort: EffortLevel,
    pub dependencies: Vec<String>,
}

impl TryFromState for Plan {
    fn try_from_state(key: &ArtifactKey, state: &StateEnvelope) -> Result<Self, AdapterError> {
        let json: serde_json::Value = serde_json::Value::try_from_state(key, state)?;
        serde_json::from_value(json.clone()).map_err(|e| AdapterError::JsonError {
            key: key.to_string(),
            error: e.to_string(),
        })
    }
}

impl IntoState for Plan {
    fn into_state(self, key: ArtifactKey, state: &mut StateEnvelope) {
        let json = serde_json::to_value(&self).unwrap();
        json.into_state(key, state);
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum EffortLevel {
    Trivial,
    Moderate,
    Significant,
}

pub struct PlanStep<S: Services> {
    input_key: ArtifactKey,
    output_key: ArtifactKey,
    _phantom: PhantomData<S>,
}

impl<S: Services> PlanStep<S> {
    pub fn new() -> Self {
        Self {
            input_key: ArtifactKey::new("scope"),
            output_key: ArtifactKey::new("plan"),
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

impl<S: Services> Default for PlanStep<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Services> Transformer for PlanStep<S> {
    type Services = S;

    fn name(&self) -> &'static str {
        "plan"
    }

    fn transform(
        &self,
        ctx: &mut ExecCtx<Self::Services>,
        mut state: StateEnvelope,
    ) -> Result<StateEnvelope, StepError> {
        let scope: ScopeAnalysis = get_typed(&self.input_key, &state).map_err(|e| {
            StepError::transformer(
                "plan",
                format!(
                    "Failed to get scope from artifact key '{}': {}",
                    self.input_key, e
                ),
            )
        })?;

        let plan: Plan = call_json(
            ctx,
            self.name(),
            format!(
                "Return JSON only with keys 'steps', 'estimated_effort', and 'dependencies'. Use one of Trivial, Moderate, or Significant for estimated_effort. Build a concise execution plan for this scope: {}",
                serde_json::to_string(&scope).unwrap_or_default()
            ),
        )?;

        put_typed(self.output_key.clone(), plan, &mut state);

        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scope::{Complexity, ScopeType};
    use crate::test_services::{JsonSequenceServices, NoopServices};
    use naaf_schema::artifacts::ArtifactValue;
    use naaf_schema::execution_status::ExecutionStatus;
    use naaf_schema::lineage::Lineage;
    use naaf_schema::state::{RunId, StateEnvelope, StateId};
    use naaf_schema::state_kind::StateKind;

    fn make_state_with_scope(scope_type: ScopeType, complexity: Complexity) -> StateEnvelope {
        let scope = ScopeAnalysis {
            scope_type,
            keywords: vec!["test".to_string()],
            estimated_complexity: complexity,
        };
        let mut state = StateEnvelope::new(
            StateId::new(),
            RunId::new(),
            StateKind::Proposed,
            Lineage::new(None, None, ExecutionStatus::Pending),
        );
        state.artifacts.insert(
            ArtifactKey::new("scope"),
            ArtifactValue::json(serde_json::json!(scope)),
        );
        state
    }

    fn make_ctx(response: &'static str) -> ExecCtx<JsonSequenceServices> {
        ExecCtx::new(RunId::new(), JsonSequenceServices::from_json([response]))
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_plan_creates_file_system_plan() {
        let plan = PlanStep::new();
        let mut ctx = make_ctx(
            r#"{"steps":["Validate file path","Check permissions"],"estimated_effort":"Trivial","dependencies":["filesystem access"]}"#,
        );
        let state = make_state_with_scope(ScopeType::FileSystem, Complexity::Low);

        let result = plan.transform(&mut ctx, state).unwrap();
        let plan_result: Plan = get_typed(&ArtifactKey::new("plan"), &result).unwrap();
        assert!(
            plan_result
                .steps
                .contains(&"Validate file path".to_string())
        );
        assert_eq!(plan_result.estimated_effort, EffortLevel::Trivial);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_plan_creates_code_analysis_plan() {
        let plan = PlanStep::new();
        let mut ctx = make_ctx(
            r#"{"steps":["Parse code structure","Analyze dependencies"],"estimated_effort":"Moderate","dependencies":["source code access"]}"#,
        );
        let state = make_state_with_scope(ScopeType::CodeAnalysis, Complexity::Medium);

        let result = plan.transform(&mut ctx, state).unwrap();
        let plan_result: Plan = get_typed(&ArtifactKey::new("plan"), &result).unwrap();
        assert!(
            plan_result
                .steps
                .contains(&"Parse code structure".to_string())
        );
        assert_eq!(plan_result.estimated_effort, EffortLevel::Moderate);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_plan_creates_testing_plan() {
        let plan = PlanStep::new();
        let mut ctx = make_ctx(
            r#"{"steps":["Identify test targets","Execute tests"],"estimated_effort":"Significant","dependencies":["test framework"]}"#,
        );
        let state = make_state_with_scope(ScopeType::Testing, Complexity::High);

        let result = plan.transform(&mut ctx, state).unwrap();
        let plan_result: Plan = get_typed(&ArtifactKey::new("plan"), &result).unwrap();
        assert!(plan_result.steps.contains(&"Execute tests".to_string()));
        assert_eq!(plan_result.estimated_effort, EffortLevel::Significant);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_plan_custom_keys() {
        let plan = PlanStep::with_keys("analysis", "result");
        let scope = ScopeAnalysis {
            scope_type: ScopeType::General,
            keywords: vec![],
            estimated_complexity: Complexity::Low,
        };
        let mut state = StateEnvelope::new(
            StateId::new(),
            RunId::new(),
            StateKind::Proposed,
            Lineage::new(None, None, ExecutionStatus::Pending),
        );
        state.artifacts.insert(
            ArtifactKey::new("analysis"),
            ArtifactValue::json(serde_json::json!(scope)),
        );

        let mut ctx = make_ctx(
            r#"{"steps":["Do the thing"],"estimated_effort":"Trivial","dependencies":[]}"#,
        );
        let result = plan.transform(&mut ctx, state).unwrap();

        let plan_result: Plan = get_typed(&ArtifactKey::new("result"), &result).unwrap();
        assert!(!plan_result.steps.is_empty());
    }

    #[test]
    fn test_plan_missing_runtime_fails() {
        let plan = PlanStep::new();
        let mut ctx = ExecCtx::new(RunId::new(), NoopServices);
        let state = make_state_with_scope(ScopeType::FileSystem, Complexity::Low);

        let result = plan.transform(&mut ctx, state);
        assert!(result.is_err());
    }
}
