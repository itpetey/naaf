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
//! use workflow_builtins::ScopeStep;
//! use workflow_core::steps::Transformer;
//!
//! let scope_step = ScopeStep::new();
//! // Transform state with "normalized" artifact to get "scope" artifact
//! ```

use serde::{Deserialize, Serialize};
use workflow_core::budget::{DummyServices, ExecCtx};
use workflow_core::errors::StepError;
use workflow_core::steps::Transformer;
use workflow_schema::adapters::{AdapterError, IntoState, TryFromState, get_typed, put_typed};
use workflow_schema::artifacts::ArtifactKey;
use workflow_schema::state::StateEnvelope;

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

pub struct ScopeStep {
    input_key: ArtifactKey,
    output_key: ArtifactKey,
}

impl ScopeStep {
    pub fn new() -> Self {
        Self {
            input_key: ArtifactKey::new("normalized"),
            output_key: ArtifactKey::new("scope"),
        }
    }

    pub fn with_keys(input_key: impl Into<String>, output_key: impl Into<String>) -> Self {
        Self {
            input_key: ArtifactKey::new(input_key),
            output_key: ArtifactKey::new(output_key),
        }
    }

    fn analyze_scope(normalized: &str) -> ScopeAnalysis {
        let keywords = Self::extract_keywords(normalized);
        let scope_type = Self::determine_scope_type(&keywords);
        let estimated_complexity = Self::estimate_complexity(normalized, &keywords);

        ScopeAnalysis {
            scope_type,
            keywords,
            estimated_complexity,
        }
    }

    fn extract_keywords(input: &str) -> Vec<String> {
        let stop_words = [
            "a", "an", "the", "is", "are", "was", "were", "be", "been", "being", "have", "has",
            "had", "do", "does", "did", "will", "would", "could", "should", "may", "might", "must",
            "shall", "can", "need", "to", "in", "on", "at", "for", "with", "by", "from", "as",
            "into", "through",
        ];

        input
            .split_whitespace()
            .filter(|word| !stop_words.contains(&word.to_lowercase().as_str()))
            .filter(|word| word.len() > 2)
            .map(|s| s.to_string())
            .collect()
    }

    fn determine_scope_type(keywords: &[String]) -> ScopeType {
        let file_keywords = [
            "file",
            "create",
            "read",
            "write",
            "delete",
            "directory",
            "folder",
        ];
        let code_keywords = [
            "function",
            "class",
            "method",
            "variable",
            "refactor",
            "implement",
            "bug",
            "fix",
        ];
        let test_keywords = ["test", "testing", "spec", "verify", "check", "assert"];
        let doc_keywords = ["document", "documentation", "readme", "comment", "explain"];

        let all_keywords_lower: Vec<String> = keywords.iter().map(|k| k.to_lowercase()).collect();

        if all_keywords_lower
            .iter()
            .any(|k| test_keywords.contains(&k.as_str()))
        {
            ScopeType::Testing
        } else if all_keywords_lower
            .iter()
            .any(|k| file_keywords.contains(&k.as_str()))
        {
            ScopeType::FileSystem
        } else if all_keywords_lower
            .iter()
            .any(|k| code_keywords.contains(&k.as_str()))
        {
            ScopeType::CodeAnalysis
        } else if all_keywords_lower
            .iter()
            .any(|k| doc_keywords.contains(&k.as_str()))
        {
            ScopeType::Documentation
        } else {
            ScopeType::General
        }
    }

    fn estimate_complexity(input: &str, keywords: &[String]) -> Complexity {
        let word_count = input.split_whitespace().count();
        let keyword_count = keywords.len();

        if word_count <= 5 && keyword_count <= 3 {
            Complexity::Low
        } else if word_count <= 15 && keyword_count <= 8 {
            Complexity::Medium
        } else {
            Complexity::High
        }
    }
}

impl Default for ScopeStep {
    fn default() -> Self {
        Self::new()
    }
}

impl Transformer for ScopeStep {
    type Services = DummyServices;

    fn name(&self) -> &'static str {
        "scope"
    }

    fn transform(
        &self,
        _ctx: &mut ExecCtx<Self::Services>,
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

        let scope_analysis = Self::analyze_scope(&normalized_input.normalized);

        put_typed(self.output_key.clone(), scope_analysis, &mut state);

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

    fn make_ctx() -> ExecCtx<DummyServices> {
        ExecCtx::new(RunId::new(), DummyServices)
    }

    #[test]
    fn test_scope_analyzes_file_operations() {
        let scope = ScopeStep::new();
        let mut ctx = make_ctx();
        let state = make_state_with_normalized("create file");

        let result = scope.transform(&mut ctx, state).unwrap();
        let analysis: ScopeAnalysis = get_typed(&ArtifactKey::new("scope"), &result).unwrap();
        assert_eq!(analysis.scope_type, ScopeType::FileSystem);
    }

    #[test]
    fn test_scope_analyzes_code_operations() {
        let scope = ScopeStep::new();
        let mut ctx = make_ctx();
        let state = make_state_with_normalized("implement function");

        let result = scope.transform(&mut ctx, state).unwrap();
        let analysis: ScopeAnalysis = get_typed(&ArtifactKey::new("scope"), &result).unwrap();
        assert_eq!(analysis.scope_type, ScopeType::CodeAnalysis);
    }

    #[test]
    fn test_scope_analyzes_test_operations() {
        let scope = ScopeStep::new();
        let mut ctx = make_ctx();
        let state = make_state_with_normalized("write test");

        let result = scope.transform(&mut ctx, state).unwrap();
        let analysis: ScopeAnalysis = get_typed(&ArtifactKey::new("scope"), &result).unwrap();
        assert_eq!(analysis.scope_type, ScopeType::Testing);
    }

    #[test]
    fn test_scope_estimates_complexity() {
        let scope = ScopeStep::new();

        let mut ctx = make_ctx();
        let state = make_state_with_normalized("create file");
        let result = scope.transform(&mut ctx, state).unwrap();
        let analysis: ScopeAnalysis = get_typed(&ArtifactKey::new("scope"), &result).unwrap();
        assert_eq!(analysis.estimated_complexity, Complexity::Low);

        let mut ctx = make_ctx();
        let state = make_state_with_normalized(
            "implement a function that processes data and handles errors",
        );
        let result = scope.transform(&mut ctx, state).unwrap();
        let analysis: ScopeAnalysis = get_typed(&ArtifactKey::new("scope"), &result).unwrap();
        assert_eq!(analysis.estimated_complexity, Complexity::Medium);
    }

    #[test]
    fn test_scope_custom_keys() {
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

        let mut ctx = make_ctx();
        let result = scope.transform(&mut ctx, state).unwrap();

        let analysis: ScopeAnalysis = get_typed(&ArtifactKey::new("result"), &result).unwrap();
        assert!(!analysis.keywords.is_empty());
    }
}
