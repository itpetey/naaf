//! Scope step transformer for workflow systems.
//!
//! This module provides scope analysis of normalized input, determining
//! the type of request and estimating complexity.
//!
//! # Artifact Flow
//! - Reads from: `normalized` (NormalizedInput from NormalizeStep)
//! - Writes to: `scope` (ScopeAnalysis with scope type and complexity)
//!
//! # Example
//!
//! ```ignore
//! use naaf_openspec::ScopeStep;
//! use naaf_core::steps::Transformer;
//!
//! let scope_step = ScopeStep::new();
//! // Transform state with "normalized" artifact to get "scope" artifact
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
use crate::normalize::NormalizedInput;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ScopeAnalysis {
    pub scope_type: ScopeType,
    pub keywords: Vec<String>,
    pub estimated_complexity: Complexity,
}

impl TryFromState for ScopeAnalysis {
    fn try_from_state(key: &ArtifactKey, state: &StateEnvelope) -> Result<Self, AdapterError> {
        let json: serde_json::Value = serde_json::Value::try_from_state(key, state)?;
        serde_json::from_value(json.clone()).map_err(|e| AdapterError::JsonError {
            key: key.to_string(),
            error: e.to_string(),
        })
    }
}

impl IntoState for ScopeAnalysis {
    fn into_state(self, key: ArtifactKey, state: &mut StateEnvelope) {
        let json = serde_json::to_value(&self).unwrap();
        json.into_state(key, state);
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum ScopeType {
    FileSystem,
    CodeAnalysis,
    Testing,
    Documentation,
    General,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum Complexity {
    Low,
    Medium,
    High,
}

pub struct ScopeStep<S: Services> {
    input_key: ArtifactKey,
    output_key: ArtifactKey,
    _phantom: PhantomData<S>,
}

impl<S: Services> ScopeStep<S> {
    pub fn new() -> Self {
        Self {
            input_key: ArtifactKey::new("normalized"),
            output_key: ArtifactKey::new("scope"),
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

impl<S: Services> Default for ScopeStep<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Services> Transformer for ScopeStep<S> {
    type Services = S;

    fn name(&self) -> &'static str {
        "scope"
    }

    fn transform(
        &self,
        ctx: &mut ExecCtx<Self::Services>,
        mut state: StateEnvelope,
    ) -> Result<StateEnvelope, StepError> {
        let normalized_input: NormalizedInput =
            get_typed(&self.input_key, &state).map_err(|e| {
                StepError::transformer(
                    "scope",
                    format!(
                        "Failed to get normalized input from artifact key '{}': {}",
                        self.input_key, e
                    ),
                )
            })?;

        let scope_analysis: ScopeAnalysis = call_json(
            ctx,
            self.name(),
            format!(
                "Return JSON only with keys 'scope_type', 'keywords', and 'estimated_complexity'. Use one of FileSystem, CodeAnalysis, Testing, Documentation, or General for scope_type, and one of Low, Medium, or High for estimated_complexity. Analyse this request: {}",
                normalized_input.normalized
            ),
        )?;

        put_typed(self.output_key.clone(), scope_analysis, &mut state);

        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_services::{JsonSequenceServices, NoopServices};
    use naaf_schema::artifacts::ArtifactValue;
    use naaf_schema::execution_status::ExecutionStatus;
    use naaf_schema::lineage::Lineage;
    use naaf_schema::state::{RunId, StateEnvelope, StateId};
    use naaf_schema::state_kind::StateKind;

    fn make_state_with_normalized(input: &str) -> StateEnvelope {
        let normalized_input = NormalizedInput {
            original: input.to_string(),
            normalized: input.to_lowercase(),
        };
        let mut state = StateEnvelope::new(
            StateId::new(),
            RunId::new(),
            StateKind::Proposed,
            Lineage::new(None, None, ExecutionStatus::Pending),
        );
        state.artifacts.insert(
            ArtifactKey::new("normalized"),
            ArtifactValue::json(serde_json::json!(normalized_input)),
        );
        state
    }

    fn make_ctx(response: &'static str) -> ExecCtx<JsonSequenceServices> {
        ExecCtx::new(RunId::new(), JsonSequenceServices::from_json([response]))
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_scope_analyzes_file_operations() {
        let scope = ScopeStep::new();
        let mut ctx = make_ctx(
            r#"{"scope_type":"FileSystem","keywords":["create","file"],"estimated_complexity":"Low"}"#,
        );
        let state = make_state_with_normalized("create file");

        let result = scope.transform(&mut ctx, state).unwrap();
        let analysis: ScopeAnalysis = get_typed(&ArtifactKey::new("scope"), &result).unwrap();
        assert_eq!(analysis.scope_type, ScopeType::FileSystem);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_scope_analyzes_code_operations() {
        let scope = ScopeStep::new();
        let mut ctx = make_ctx(
            r#"{"scope_type":"CodeAnalysis","keywords":["implement","function"],"estimated_complexity":"Medium"}"#,
        );
        let state = make_state_with_normalized("implement function");

        let result = scope.transform(&mut ctx, state).unwrap();
        let analysis: ScopeAnalysis = get_typed(&ArtifactKey::new("scope"), &result).unwrap();
        assert_eq!(analysis.scope_type, ScopeType::CodeAnalysis);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_scope_analyzes_test_operations() {
        let scope = ScopeStep::new();
        let mut ctx = make_ctx(
            r#"{"scope_type":"Testing","keywords":["write","test"],"estimated_complexity":"Medium"}"#,
        );
        let state = make_state_with_normalized("write test");

        let result = scope.transform(&mut ctx, state).unwrap();
        let analysis: ScopeAnalysis = get_typed(&ArtifactKey::new("scope"), &result).unwrap();
        assert_eq!(analysis.scope_type, ScopeType::Testing);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_scope_estimates_complexity() {
        let scope = ScopeStep::new();

        let mut ctx = make_ctx(
            r#"{"scope_type":"FileSystem","keywords":["create","file"],"estimated_complexity":"Low"}"#,
        );
        let state = make_state_with_normalized("create file");
        let result = scope.transform(&mut ctx, state).unwrap();
        let analysis: ScopeAnalysis = get_typed(&ArtifactKey::new("scope"), &result).unwrap();
        assert_eq!(analysis.estimated_complexity, Complexity::Low);

        let mut ctx = make_ctx(
            r#"{"scope_type":"CodeAnalysis","keywords":["implement","function","processes","data","handles","errors"],"estimated_complexity":"Medium"}"#,
        );
        let state = make_state_with_normalized(
            "implement a function that processes data and handles errors",
        );
        let result = scope.transform(&mut ctx, state).unwrap();
        let analysis: ScopeAnalysis = get_typed(&ArtifactKey::new("scope"), &result).unwrap();
        assert_eq!(analysis.estimated_complexity, Complexity::Medium);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_scope_custom_keys() {
        let scope = ScopeStep::with_keys("norm", "result");
        let normalized_input = NormalizedInput {
            original: "Test input".to_string(),
            normalized: "test input".to_string(),
        };
        let mut state = StateEnvelope::new(
            StateId::new(),
            RunId::new(),
            StateKind::Proposed,
            Lineage::new(None, None, ExecutionStatus::Pending),
        );
        state.artifacts.insert(
            ArtifactKey::new("norm"),
            ArtifactValue::json(serde_json::json!(normalized_input)),
        );

        let mut ctx = make_ctx(
            r#"{"scope_type":"General","keywords":["test","input"],"estimated_complexity":"Low"}"#,
        );
        let result = scope.transform(&mut ctx, state).unwrap();

        let analysis: ScopeAnalysis = get_typed(&ArtifactKey::new("result"), &result).unwrap();
        assert!(!analysis.keywords.is_empty());
    }

    #[test]
    fn test_scope_missing_runtime_fails() {
        let scope = ScopeStep::new();
        let mut ctx = ExecCtx::new(RunId::new(), NoopServices);
        let state = make_state_with_normalized("create file");

        let result = scope.transform(&mut ctx, state);
        assert!(result.is_err());
    }
}
