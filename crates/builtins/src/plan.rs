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
//! use naaf_builtins::PlanStep;
//! use naaf_core::steps::Transformer;
//!
//! let plan_step = PlanStep::new();
//! // Transform state with "scope" artifact to get "plan" artifact
//! ```

use naaf_core::budget::{DummyServices, ExecCtx};
use naaf_core::errors::StepError;
use naaf_core::steps::Transformer;
use naaf_schema::adapters::{AdapterError, IntoState, TryFromState, get_typed, put_typed};
use naaf_schema::artifacts::ArtifactKey;
use naaf_schema::state::StateEnvelope;
use serde::{Deserialize, Serialize};

use crate::scope::{Complexity, ScopeAnalysis, ScopeType};

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

pub struct PlanStep {
    input_key: ArtifactKey,
    output_key: ArtifactKey,
}

impl PlanStep {
    pub fn new() -> Self {
        Self {
            input_key: ArtifactKey::new("scope"),
            output_key: ArtifactKey::new("plan"),
        }
    }

    pub fn with_keys(input_key: impl Into<String>, output_key: impl Into<String>) -> Self {
        Self {
            input_key: ArtifactKey::new(input_key),
            output_key: ArtifactKey::new(output_key),
        }
    }

    fn create_plan(scope: &ScopeAnalysis) -> Plan {
        let steps = Self::generate_steps(scope);
        let estimated_effort = Self::determine_effort(scope);
        let dependencies = Self::identify_dependencies(scope);

        Plan {
            steps,
            estimated_effort,
            dependencies,
        }
    }

    fn generate_steps(scope: &ScopeAnalysis) -> Vec<String> {
        match scope.scope_type {
            ScopeType::FileSystem => {
                vec![
                    "Validate file path".to_string(),
                    "Check permissions".to_string(),
                    "Execute file operation".to_string(),
                    "Verify result".to_string(),
                ]
            }
            ScopeType::CodeAnalysis => {
                vec![
                    "Parse code structure".to_string(),
                    "Analyze dependencies".to_string(),
                    "Identify changes needed".to_string(),
                    "Generate modifications".to_string(),
                    "Validate changes".to_string(),
                ]
            }
            ScopeType::Testing => {
                vec![
                    "Identify test targets".to_string(),
                    "Set up test environment".to_string(),
                    "Execute tests".to_string(),
                    "Report results".to_string(),
                ]
            }
            ScopeType::Documentation => {
                vec![
                    "Gather source material".to_string(),
                    "Structure documentation".to_string(),
                    "Write content".to_string(),
                    "Review and refine".to_string(),
                ]
            }
            ScopeType::General => {
                vec![
                    "Understand request".to_string(),
                    "Plan execution".to_string(),
                    "Carry out steps".to_string(),
                    "Verify completion".to_string(),
                ]
            }
        }
    }

    fn determine_effort(scope: &ScopeAnalysis) -> EffortLevel {
        match scope.estimated_complexity {
            Complexity::Low => EffortLevel::Trivial,
            Complexity::Medium => EffortLevel::Moderate,
            Complexity::High => EffortLevel::Significant,
        }
    }

    fn identify_dependencies(scope: &ScopeAnalysis) -> Vec<String> {
        match scope.scope_type {
            ScopeType::FileSystem => vec!["filesystem access".to_string()],
            ScopeType::CodeAnalysis => vec![
                "source code access".to_string(),
                "language parser".to_string(),
            ],
            ScopeType::Testing => vec!["test framework".to_string(), "test data".to_string()],
            ScopeType::Documentation => vec!["source context".to_string()],
            ScopeType::General => vec![],
        }
    }
}

impl Default for PlanStep {
    fn default() -> Self {
        Self::new()
    }
}

impl Transformer for PlanStep {
    type Services = DummyServices;

    fn name(&self) -> &'static str {
        "plan"
    }

    fn transform(
        &self,
        _ctx: &mut ExecCtx<Self::Services>,
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

        let plan = Self::create_plan(&scope);

        put_typed(self.output_key.clone(), plan, &mut state);

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

    fn make_ctx() -> ExecCtx<DummyServices> {
        ExecCtx::new(RunId::new(), DummyServices)
    }

    #[test]
    fn test_plan_creates_file_system_plan() {
        let plan = PlanStep::new();
        let mut ctx = make_ctx();
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

    #[test]
    fn test_plan_creates_code_analysis_plan() {
        let plan = PlanStep::new();
        let mut ctx = make_ctx();
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

    #[test]
    fn test_plan_creates_testing_plan() {
        let plan = PlanStep::new();
        let mut ctx = make_ctx();
        let state = make_state_with_scope(ScopeType::Testing, Complexity::High);

        let result = plan.transform(&mut ctx, state).unwrap();
        let plan_result: Plan = get_typed(&ArtifactKey::new("plan"), &result).unwrap();
        assert!(plan_result.steps.contains(&"Execute tests".to_string()));
        assert_eq!(plan_result.estimated_effort, EffortLevel::Significant);
    }

    #[test]
    fn test_plan_custom_keys() {
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

        let mut ctx = make_ctx();
        let result = plan.transform(&mut ctx, state).unwrap();

        let plan_result: Plan = get_typed(&ArtifactKey::new("result"), &result).unwrap();
        assert!(!plan_result.steps.is_empty());
    }
}
