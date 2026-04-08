//! Input classification transformers for workflow systems.
//!
//! This module provides tools for classifying user input into categories
//! (greeting, actionable, ambiguous) with confidence scores.

use std::marker::PhantomData;

use naaf_core::budget::{ExecCtx, Services};
use naaf_core::errors::StepError;
use naaf_core::steps::Transformer;
use naaf_schema::adapters::{AdapterError, IntoState, TryFromState, get_typed, put_typed};
use naaf_schema::artifacts::ArtifactKey;
use naaf_schema::state::StateEnvelope;
use serde::{Deserialize, Serialize};

// Confidence thresholds based on pattern matching quality.
// These values are heuristic but provide reasonable discrimination for v1.

/// High confidence for exact pattern matches (e.g., "hi" exactly matches greeting pattern).
const CONFIDENCE_EXACT_MATCH: f64 = 0.95;

/// Medium-high confidence for partial pattern matches (e.g., "hi there" contains greeting).
const CONFIDENCE_PATTERN_MATCH: f64 = 0.85;

/// High confidence for clear actionable intent (e.g., "create a file").
const CONFIDENCE_ACTION_VERB: f64 = 0.90;

/// High confidence for short ambiguous input (1-2 chars).
const CONFIDENCE_SHORT_AMBIGUOUS: f64 = 0.90;

/// Medium confidence for modal ambiguity (e.g., "could you help").
const CONFIDENCE_MODAL_AMBIGUOUS: f64 = 0.70;

/// Default low confidence when unable to determine classification.
const CONFIDENCE_DEFAULT_AMBIGUOUS: f64 = 0.60;

/// Classification category for user input.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum InputClass {
    /// Casual greeting (e.g., "hi", "hello", "hey").
    Greeting,
    /// Clear, actionable request (e.g., "create a file", "fix bug 123").
    Actionable,
    /// Ambiguous or unclear request needing clarification.
    Ambiguous,
}

/// Result of input classification containing the category and confidence score.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Classification {
    /// The determined classification category.
    pub class: InputClass,
    /// Confidence score from 0.0 to1.0 indicating classification certainty.
    pub confidence: f64,
}

impl TryFromState for Classification {
    fn try_from_state(key: &ArtifactKey, state: &StateEnvelope) -> Result<Self, AdapterError> {
        let json: serde_json::Value = serde_json::Value::try_from_state(key, state)?;
        serde_json::from_value(json.clone()).map_err(|e| AdapterError::JsonError {
            key: key.to_string(),
            error: e.to_string(),
        })
    }
}

impl IntoState for Classification {
    fn into_state(self, key: ArtifactKey, state: &mut StateEnvelope) {
        let json = serde_json::to_value(&self).unwrap();
        json.into_state(key, state);
    }
}

/// Transformer that classifies user input into greeting/actionable/ambiguous categories.
///
/// # Input
/// Reads user input from the artifact key specified by `input_key` (default: "input").
///
/// # Output
/// Writes a `Classification` to the artifact key specified by `output_key` (default: "classification").
///
/// # Classification Strategy
///
/// The classifier uses pattern matching in the following order:
///
/// 1. **Greetings** (confidence: 0.85-0.95): Exact or near-exact matches against common greetings
/// 2. **Short input** (<3 chars, confidence: 0.90): Labeled ambiguous as likely unclear
/// 3. **Action verbs** (confidence: 0.90): Input containing clear action verbs like "create", "fix", "list"
/// 4. **Modal words** (confidence: 0.70): Input with modal words like "could", "would" marked ambiguous
/// 5. **Intent patterns** (confidence: 0.85): Patterns like "I want...", "I need..." marked actionable
/// 6. **Default** (confidence: 0.60): Everything else marked ambiguous
///
/// # Example
///
/// ```ignore
/// use naaf_openspec::ClassifyInput;
/// use naaf_core::steps::Transformer;
///
/// let classifier = ClassifyInput::new();
/// // Transform state with "input" artifact to get "classification" artifact
/// ```
pub struct ClassifyInput<S: Services> {
    input_key: ArtifactKey,
    output_key: ArtifactKey,
    _phantom: PhantomData<S>,
}

impl<S: Services> ClassifyInput<S> {
    /// Creates a new ClassifyInput transformer with default artifact keys.
    ///
    /// Uses `"input"` as the input key and `"classification"` as the output key.
    pub fn new() -> Self {
        Self {
            input_key: ArtifactKey::new("input"),
            output_key: ArtifactKey::new("classification"),
            _phantom: PhantomData,
        }
    }

    /// Creates a new ClassifyInput transformer with custom artifact keys.
    pub fn with_keys(input_key: impl Into<String>, output_key: impl Into<String>) -> Self {
        Self {
            input_key: ArtifactKey::new(input_key),
            output_key: ArtifactKey::new(output_key),
            _phantom: PhantomData,
        }
    }

    fn classify(&self, input: &str) -> Classification {
        let input_lower = input.to_lowercase();
        let trimmed = input_lower.trim();

        let greeting_patterns = [
            "hi",
            "hello",
            "hey",
            "good morning",
            "good afternoon",
            "good evening",
            "good night",
            "howdy",
            "greetings",
            "yo",
            "sup",
        ];

        for pattern in greeting_patterns.iter() {
            if trimmed == *pattern
                || trimmed.starts_with(&format!("{} ", pattern))
                || trimmed.ends_with(&format!(" {}", pattern))
            {
                let confidence = if trimmed == *pattern {
                    CONFIDENCE_EXACT_MATCH
                } else {
                    CONFIDENCE_PATTERN_MATCH
                };
                return Classification {
                    class: InputClass::Greeting,
                    confidence,
                };
            }
        }

        if trimmed.len() <= 2 && !trimmed.is_empty() {
            return Classification {
                class: InputClass::Ambiguous,
                confidence: CONFIDENCE_SHORT_AMBIGUOUS,
            };
        }

        let action_verbs = [
            "create",
            "build",
            "write",
            "fix",
            "add",
            "remove",
            "delete",
            "update",
            "impl",
            "implement",
            "refactor",
            "test",
            "run",
            "list",
            "show",
            "get",
            "find",
            "search",
            "help",
            "explain",
            "analyze",
            "review",
        ];

        let modal_words = ["could", "would", "should", "can", "might", "may"];

        for verb in action_verbs.iter() {
            if trimmed.starts_with(&format!("{} ", verb))
                || trimmed.contains(&format!(" {} ", verb))
            {
                return Classification {
                    class: InputClass::Actionable,
                    confidence: CONFIDENCE_ACTION_VERB,
                };
            }
        }

        for modal in modal_words.iter() {
            if trimmed.contains(&format!(" {} ", modal)) {
                return Classification {
                    class: InputClass::Ambiguous,
                    confidence: CONFIDENCE_MODAL_AMBIGUOUS,
                };
            }
        }

        if trimmed.starts_with("i want") || trimmed.starts_with("i need") {
            return Classification {
                class: InputClass::Actionable,
                confidence: CONFIDENCE_PATTERN_MATCH,
            };
        }

        if trimmed.contains('?') {
            return Classification {
                class: InputClass::Ambiguous,
                confidence: CONFIDENCE_MODAL_AMBIGUOUS,
            };
        }

        Classification {
            class: InputClass::Ambiguous,
            confidence: CONFIDENCE_DEFAULT_AMBIGUOUS,
        }
    }
}

impl<S: Services> Default for ClassifyInput<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Services> Transformer for ClassifyInput<S> {
    type Services = S;

    fn name(&self) -> &'static str {
        "classify_input"
    }

    fn transform(
        &self,
        _ctx: &mut ExecCtx<Self::Services>,
        mut state: StateEnvelope,
    ) -> Result<StateEnvelope, StepError> {
        let input: String = get_typed(&self.input_key, &state).map_err(|e| {
            StepError::transformer(
                "classify_input",
                format!(
                    "Failed to get input from artifact key '{}': {}",
                    self.input_key, e
                ),
            )
        })?;

        let classification = self.classify(&input);

        put_typed(self.output_key.clone(), classification.clone(), &mut state);

        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_services::NoopServices;
    use naaf_schema::artifacts::ArtifactValue;
    use naaf_schema::execution_status::ExecutionStatus;
    use naaf_schema::lineage::Lineage;
    use naaf_schema::state::{RunId, StateId};
    use naaf_schema::state_kind::StateKind;

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

    fn make_state_without_input() -> StateEnvelope {
        StateEnvelope::new(
            StateId::new(),
            RunId::new(),
            StateKind::Proposed,
            Lineage::new(None, None, ExecutionStatus::Pending),
        )
    }

    fn make_ctx() -> ExecCtx<NoopServices> {
        ExecCtx::new(RunId::new(), NoopServices)
    }

    #[test]
    fn test_greeting_classification() {
        let classifier = ClassifyInput::new();
        let mut ctx = make_ctx();

        for greeting in &["Hi", "Hello", "Hey", "hi", "hello", "hey"] {
            let state = make_state_with_input(greeting);
            let result = classifier.transform(&mut ctx, state).unwrap();
            let classification: Classification =
                get_typed(&ArtifactKey::new("classification"), &result).unwrap();
            assert_eq!(classification.class, InputClass::Greeting);
            assert!(classification.confidence >= CONFIDENCE_PATTERN_MATCH);
        }
    }

    #[test]
    fn test_actionable_classification() {
        let classifier = ClassifyInput::new();
        let mut ctx = make_ctx();

        for input in &[
            "Create a file",
            "Build the project",
            "Fix the bug",
            "List files",
        ] {
            let state = make_state_with_input(input);
            let result = classifier.transform(&mut ctx, state).unwrap();
            let classification: Classification =
                get_typed(&ArtifactKey::new("classification"), &result).unwrap();
            assert_eq!(classification.class, InputClass::Actionable);
            assert!(classification.confidence >= CONFIDENCE_ACTION_VERB);
        }
    }

    #[test]
    fn test_ambiguous_classification() {
        let classifier = ClassifyInput::new();
        let mut ctx = make_ctx();

        for input in &["Could you help", "I might want", "What about"] {
            let state = make_state_with_input(input);
            let result = classifier.transform(&mut ctx, state).unwrap();
            let classification: Classification =
                get_typed(&ArtifactKey::new("classification"), &result).unwrap();
            assert_eq!(classification.class, InputClass::Ambiguous);
        }
    }

    #[test]
    fn test_short_input_ambiguous() {
        let classifier = ClassifyInput::new();
        let mut ctx = make_ctx();

        let state = make_state_with_input("Hi");
        let result = classifier.transform(&mut ctx, state).unwrap();
        let classification: Classification =
            get_typed(&ArtifactKey::new("classification"), &result).unwrap();

        assert_eq!(classification.class, InputClass::Greeting);
        assert!(classification.confidence >= CONFIDENCE_EXACT_MATCH);
    }

    #[test]
    fn test_custom_keys() {
        let classifier = ClassifyInput::with_keys("text", "result");
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
        let result = classifier.transform(&mut ctx, state).unwrap();

        let classification: Classification =
            get_typed(&ArtifactKey::new("result"), &result).unwrap();
        assert_eq!(classification.class, InputClass::Greeting);
    }

    #[test]
    fn test_missing_input_artifact() {
        let classifier = ClassifyInput::new();
        let mut ctx = make_ctx();
        let state = make_state_without_input();

        let result = classifier.transform(&mut ctx, state);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_input() {
        let classifier = ClassifyInput::new();
        let mut ctx = make_ctx();
        let state = make_state_with_input("");

        let result = classifier.transform(&mut ctx, state).unwrap();
        let classification: Classification =
            get_typed(&ArtifactKey::new("classification"), &result).unwrap();

        assert_eq!(classification.class, InputClass::Ambiguous);
    }

    #[test]
    fn test_whitespace_only_input() {
        let classifier = ClassifyInput::new();
        let mut ctx = make_ctx();
        let state = make_state_with_input("   ");

        let result = classifier.transform(&mut ctx, state).unwrap();
        let classification: Classification =
            get_typed(&ArtifactKey::new("classification"), &result).unwrap();

        assert_eq!(classification.class, InputClass::Ambiguous);
    }

    #[test]
    fn test_single_char_input() {
        let classifier = ClassifyInput::new();
        let mut ctx = make_ctx();
        let state = make_state_with_input("a");

        let result = classifier.transform(&mut ctx, state).unwrap();
        let classification: Classification =
            get_typed(&ArtifactKey::new("classification"), &result).unwrap();

        assert_eq!(classification.class, InputClass::Ambiguous);
        assert!((classification.confidence - CONFIDENCE_SHORT_AMBIGUOUS).abs() < 0.01);
    }

    #[test]
    fn test_boundary_confidence_values() {
        let classifier = ClassifyInput::new();
        let mut ctx = make_ctx();

        // Test exact greeting match (should be 0.95)
        let state = make_state_with_input("hi");
        let result = classifier.transform(&mut ctx, state).unwrap();
        let classification: Classification =
            get_typed(&ArtifactKey::new("classification"), &result).unwrap();
        assert!((classification.confidence - CONFIDENCE_EXACT_MATCH).abs() < 0.01);

        // Test action verb (should be 0.90)
        let state = make_state_with_input("create something");
        let result = classifier.transform(&mut ctx, state).unwrap();
        let classification: Classification =
            get_typed(&ArtifactKey::new("classification"), &result).unwrap();
        assert!((classification.confidence - CONFIDENCE_ACTION_VERB).abs() < 0.01);
    }
}
