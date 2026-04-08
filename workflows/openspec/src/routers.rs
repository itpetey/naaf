//! Routers for workflow classification-based routing.
//!
//! This module provides routers that make routing decisions based on
//! input classification results.

use std::marker::PhantomData;

use naaf_core::budget::{ExecCtx, Services};
use naaf_core::errors::StepError;
use naaf_core::route::RouteDecision;
use naaf_core::steps::Router;
use naaf_schema::adapters::get_typed;
use naaf_schema::artifacts::ArtifactKey;
use naaf_schema::state::StateEnvelope;

use crate::classify_input::{Classification, InputClass};

/// Router that directs inputs based on their classification category.
///
/// Routes to different workflow nodes depending on whether the input is
/// `Greeting`, `Ambiguous`, or `Actionable`.
///
/// # Input
/// Expects a `Classification` artifact at the specified key (default: "classification").
///
/// # Output
/// - `RouteDecision::Next(greeting_route)` for greeting inputs
/// - `RouteDecision::Next(clarification_route)` for ambiguous inputs
/// - `RouteDecision::Next(actionable_route)` for actionable inputs
///
/// # Example
///
/// ```ignore
/// use naaf_openspec::InputClassificationRouter;
/// use naaf_core::steps::Router;
///
/// let router = InputClassificationRouter::new("greeting_path", "clarify_path", "continue_path");
/// // Routes based on input classification
/// ```
pub struct InputClassificationRouter<S: Services> {
    classification_key: ArtifactKey,
    greeting_route: String,
    clarification_route: String,
    actionable_route: String,
    _phantom: PhantomData<S>,
}

impl<S: Services> InputClassificationRouter<S> {
    /// Creates a new router with default artifact key "classification".
    ///
    /// # Arguments
    ///
    /// * `greeting_route` - Route for greeting inputs
    /// * `clarification_route` - Route for ambiguous inputs
    /// * `actionable_route` - Route for actionable inputs
    pub fn new(
        greeting_route: impl Into<String>,
        clarification_route: impl Into<String>,
        actionable_route: impl Into<String>,
    ) -> Self {
        Self {
            classification_key: ArtifactKey::new("classification"),
            greeting_route: greeting_route.into(),
            clarification_route: clarification_route.into(),
            actionable_route: actionable_route.into(),
            _phantom: PhantomData,
        }
    }

    /// Creates a new router with a custom classification artifact key.
    pub fn with_keys(
        classification_key: impl Into<String>,
        greeting_route: impl Into<String>,
        clarification_route: impl Into<String>,
        actionable_route: impl Into<String>,
    ) -> Self {
        Self {
            classification_key: ArtifactKey::new(classification_key),
            greeting_route: greeting_route.into(),
            clarification_route: clarification_route.into(),
            actionable_route: actionable_route.into(),
            _phantom: PhantomData,
        }
    }
}

impl<S: Services> Router for InputClassificationRouter<S> {
    type Services = S;

    fn name(&self) -> &'static str {
        "input_classification_router"
    }

    fn route(
        &self,
        _ctx: &mut ExecCtx<Self::Services>,
        state: &StateEnvelope,
    ) -> Result<RouteDecision, StepError> {
        let classification: Classification =
            get_typed(&self.classification_key, state).map_err(|e| {
                StepError::router(
                    "input_classification_router",
                    format!(
                        "Failed to get classification from artifact key '{}': {}",
                        self.classification_key, e
                    ),
                )
            })?;

        match classification.class {
            InputClass::Greeting => Ok(RouteDecision::next(&self.greeting_route)),
            InputClass::Ambiguous => Ok(RouteDecision::next(&self.clarification_route)),
            InputClass::Actionable => Ok(RouteDecision::next(&self.actionable_route)),
        }
    }
}

/// Router that directs ambiguous inputs to human clarification.
///
/// Routes to a clarification workflow when the input classification is
/// `Ambiguous`. Returns `Terminal` for all other classifications, allowing
/// upstream routers to handle non-ambiguous cases.
///
/// # Input
/// Expects a `Classification` artifact at the specified key (default: "classification").
///
/// # Output
/// - `RouteDecision::Next(clarification_route)` for ambiguous inputs
/// - `RouteDecision::Terminal` for greeting or actionable inputs
///
/// # Example
///
/// ```ignore
/// use naaf_openspec::NeedsHumanClarification;
/// use naaf_core::steps::Router;
///
/// let router = NeedsHumanClarification::new("clarification_workflow");
/// // Routes ambiguous inputs to clarification, terminal for others
/// ```
pub struct NeedsHumanClarification<S: Services> {
    classification_key: ArtifactKey,
    clarification_route: String,
    _phantom: PhantomData<S>,
}

impl<S: Services> NeedsHumanClarification<S> {
    /// Creates a new router with default artifact key "classification".
    ///
    /// # Arguments
    ///
    /// * `clarification_route` - The workflow node to route ambiguous inputs to
    pub fn new(clarification_route: impl Into<String>) -> Self {
        Self {
            classification_key: ArtifactKey::new("classification"),
            clarification_route: clarification_route.into(),
            _phantom: PhantomData,
        }
    }

    /// Creates a new router with a custom classification artifact key.
    pub fn with_keys(
        classification_key: impl Into<String>,
        clarification_route: impl Into<String>,
    ) -> Self {
        Self {
            classification_key: ArtifactKey::new(classification_key),
            clarification_route: clarification_route.into(),
            _phantom: PhantomData,
        }
    }
}

impl<S: Services> Router for NeedsHumanClarification<S> {
    type Services = S;

    fn name(&self) -> &'static str {
        "needs_human_clarification"
    }

    fn route(
        &self,
        _ctx: &mut ExecCtx<Self::Services>,
        state: &StateEnvelope,
    ) -> Result<RouteDecision, StepError> {
        let classification: Classification =
            get_typed(&self.classification_key, state).map_err(|e| {
                StepError::router(
                    "needs_human_clarification",
                    format!(
                        "Failed to get classification from artifact key '{}': {}",
                        self.classification_key, e
                    ),
                )
            })?;

        if classification.class == InputClass::Ambiguous {
            Ok(RouteDecision::next(&self.clarification_route))
        } else {
            Ok(RouteDecision::terminal())
        }
    }
}

/// Router that makes routing decisions based on confidence score thresholds.
///
/// Routes to different workflow nodes depending on whether the classification
/// confidence meets or exceeds a threshold value.
///
/// # Input
/// Expects a `Classification` artifact at the specified key (default: "classification").
///
/// # Output
/// - `RouteDecision::Next(high_confidence_route)` when confidence >= threshold
/// - `RouteDecision::Next(low_confidence_route)` when confidence < threshold
///
/// # Example
///
/// ```ignore
/// use naaf_openspec::ConfidenceThresholdRouter;
/// use naaf_core::steps::Router;
///
/// let router = ConfidenceThresholdRouter::new(0.8, "high_confidence", "low_confidence");
/// // Routes based on confidence threshold of0.8
/// ```
pub struct ConfidenceThresholdRouter<S: Services> {
    classification_key: ArtifactKey,
    high_confidence_route: String,
    low_confidence_route: String,
    threshold: f64,
    _phantom: PhantomData<S>,
}

impl<S: Services> ConfidenceThresholdRouter<S> {
    /// Creates a new confidence threshold router with default artifact key "classification".
    ///
    /// # Arguments
    ///
    /// * `threshold` - Confidence threshold (0.0 to 1.0)
    /// * `high_confidence_route` - Route for confidence >= threshold
    /// * `low_confidence_route` - Route for confidence < threshold
    pub fn new(
        threshold: f64,
        high_confidence_route: impl Into<String>,
        low_confidence_route: impl Into<String>,
    ) -> Self {
        Self {
            classification_key: ArtifactKey::new("classification"),
            high_confidence_route: high_confidence_route.into(),
            low_confidence_route: low_confidence_route.into(),
            threshold,
            _phantom: PhantomData,
        }
    }

    /// Creates a new confidence threshold router with a custom classification artifact key.
    pub fn with_keys(
        classification_key: impl Into<String>,
        threshold: f64,
        high_confidence_route: impl Into<String>,
        low_confidence_route: impl Into<String>,
    ) -> Self {
        Self {
            classification_key: ArtifactKey::new(classification_key),
            high_confidence_route: high_confidence_route.into(),
            low_confidence_route: low_confidence_route.into(),
            threshold,
            _phantom: PhantomData,
        }
    }
}

impl<S: Services> Router for ConfidenceThresholdRouter<S> {
    type Services = S;

    fn name(&self) -> &'static str {
        "confidence_threshold_router"
    }

    fn route(
        &self,
        _ctx: &mut ExecCtx<Self::Services>,
        state: &StateEnvelope,
    ) -> Result<RouteDecision, StepError> {
        let classification: Classification =
            get_typed(&self.classification_key, state).map_err(|e| {
                StepError::router(
                    "confidence_threshold_router",
                    format!(
                        "Failed to get classification from artifact key '{}': {}",
                        self.classification_key, e
                    ),
                )
            })?;

        if classification.confidence >= self.threshold {
            Ok(RouteDecision::next(&self.high_confidence_route))
        } else {
            Ok(RouteDecision::next(&self.low_confidence_route))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classify_input::InputClass;
    use crate::test_services::NoopServices;
    use naaf_schema::artifacts::ArtifactValue;
    use naaf_schema::execution_status::ExecutionStatus;
    use naaf_schema::lineage::Lineage;
    use naaf_schema::state::{RunId, StateEnvelope, StateId};
    use naaf_schema::state_kind::StateKind;

    fn make_state_with_classification(class: InputClass, confidence: f64) -> StateEnvelope {
        let mut state = StateEnvelope::new(
            StateId::new(),
            RunId::new(),
            StateKind::Proposed,
            Lineage::new(None, None, ExecutionStatus::Pending),
        );
        let classification = Classification { class, confidence };
        state.artifacts.insert(
            ArtifactKey::new("classification"),
            ArtifactValue::json(serde_json::json!(classification)),
        );
        state
    }

    fn make_ctx() -> ExecCtx<NoopServices> {
        ExecCtx::new(RunId::new(), NoopServices)
    }

    #[test]
    fn test_routes_to_clarification_for_ambiguous() {
        let router = NeedsHumanClarification::new("clarification");
        let mut ctx = make_ctx();
        let state = make_state_with_classification(InputClass::Ambiguous, 0.7);

        let decision = router.route(&mut ctx, &state).unwrap();
        assert_eq!(decision.target_nodes(), vec!["clarification".to_string()]);
    }

    #[test]
    fn test_terminal_for_greeting() {
        let router = NeedsHumanClarification::new("clarification");
        let mut ctx = make_ctx();
        let state = make_state_with_classification(InputClass::Greeting, 0.95);

        let decision = router.route(&mut ctx, &state).unwrap();
        assert!(decision.is_terminal());
    }

    #[test]
    fn test_terminal_for_actionable() {
        let router = NeedsHumanClarification::new("clarification");
        let mut ctx = make_ctx();
        let state = make_state_with_classification(InputClass::Actionable, 0.9);

        let decision = router.route(&mut ctx, &state).unwrap();
        assert!(decision.is_terminal());
    }

    #[test]
    fn test_custom_keys() {
        let router = NeedsHumanClarification::with_keys("result", "ask-human");
        let mut state = StateEnvelope::new(
            StateId::new(),
            RunId::new(),
            StateKind::Proposed,
            Lineage::new(None, None, ExecutionStatus::Pending),
        );
        let classification = Classification {
            class: InputClass::Ambiguous,
            confidence: 0.65,
        };
        state.artifacts.insert(
            ArtifactKey::new("result"),
            ArtifactValue::json(serde_json::json!(classification)),
        );

        let mut ctx = make_ctx();
        let decision = router.route(&mut ctx, &state).unwrap();
        assert_eq!(decision.target_nodes(), vec!["ask-human".to_string()]);
    }

    #[test]
    fn test_confidence_router_high_confidence() {
        let router = ConfidenceThresholdRouter::new(0.8, "high-route", "low-route");
        let mut ctx = make_ctx();
        let state = make_state_with_classification(InputClass::Actionable, 0.9);

        let decision = router.route(&mut ctx, &state).unwrap();
        assert_eq!(decision.target_nodes(), vec!["high-route".to_string()]);
    }

    #[test]
    fn test_confidence_router_low_confidence() {
        let router = ConfidenceThresholdRouter::new(0.8, "high-route", "low-route");
        let mut ctx = make_ctx();
        let state = make_state_with_classification(InputClass::Ambiguous, 0.65);

        let decision = router.route(&mut ctx, &state).unwrap();
        assert_eq!(decision.target_nodes(), vec!["low-route".to_string()]);
    }

    #[test]
    fn test_confidence_router_at_threshold() {
        let router = ConfidenceThresholdRouter::new(0.75, "high-route", "low-route");
        let mut ctx = make_ctx();
        let state = make_state_with_classification(InputClass::Greeting, 0.75);

        let decision = router.route(&mut ctx, &state).unwrap();
        assert_eq!(decision.target_nodes(), vec!["high-route".to_string()]);
    }

    #[test]
    fn test_confidence_router_custom_keys() {
        let router = ConfidenceThresholdRouter::with_keys("result", 0.7, "proceed", "fallback");
        let mut state = StateEnvelope::new(
            StateId::new(),
            RunId::new(),
            StateKind::Proposed,
            Lineage::new(None, None, ExecutionStatus::Pending),
        );
        let classification = Classification {
            class: InputClass::Actionable,
            confidence: 0.85,
        };
        state.artifacts.insert(
            ArtifactKey::new("result"),
            ArtifactValue::json(serde_json::json!(classification)),
        );

        let mut ctx = make_ctx();
        let decision = router.route(&mut ctx, &state).unwrap();
        assert_eq!(decision.target_nodes(), vec!["proceed".to_string()]);
    }

    #[test]
    fn test_missing_classification_artifact() {
        let router = NeedsHumanClarification::new("clarification");
        let mut ctx = make_ctx();
        let state = StateEnvelope::new(
            StateId::new(),
            RunId::new(),
            StateKind::Proposed,
            Lineage::new(None, None, ExecutionStatus::Pending),
        );

        let result = router.route(&mut ctx, &state);
        assert!(result.is_err());
    }

    #[test]
    fn test_input_classification_router_greeting() {
        let router = InputClassificationRouter::new("greeting", "clarify", "continue");
        let mut ctx = make_ctx();
        let state = make_state_with_classification(InputClass::Greeting, 0.95);

        let decision = router.route(&mut ctx, &state).unwrap();
        assert_eq!(decision.target_nodes(), vec!["greeting".to_string()]);
    }

    #[test]
    fn test_input_classification_router_ambiguous() {
        let router = InputClassificationRouter::new("greeting", "clarify", "continue");
        let mut ctx = make_ctx();
        let state = make_state_with_classification(InputClass::Ambiguous, 0.70);

        let decision = router.route(&mut ctx, &state).unwrap();
        assert_eq!(decision.target_nodes(), vec!["clarify".to_string()]);
    }

    #[test]
    fn test_input_classification_router_actionable() {
        let router = InputClassificationRouter::new("greeting", "clarify", "continue");
        let mut ctx = make_ctx();
        let state = make_state_with_classification(InputClass::Actionable, 0.90);

        let decision = router.route(&mut ctx, &state).unwrap();
        assert_eq!(decision.target_nodes(), vec!["continue".to_string()]);
    }

    #[test]
    fn test_input_classification_router_custom_keys() {
        let router = InputClassificationRouter::with_keys("result", "hi", "ask", "proceed");
        let mut state = StateEnvelope::new(
            StateId::new(),
            RunId::new(),
            StateKind::Proposed,
            Lineage::new(None, None, ExecutionStatus::Pending),
        );
        let classification = Classification {
            class: InputClass::Actionable,
            confidence: 0.92,
        };
        state.artifacts.insert(
            ArtifactKey::new("result"),
            ArtifactValue::json(serde_json::json!(classification)),
        );

        let mut ctx = make_ctx();
        let decision = router.route(&mut ctx, &state).unwrap();
        assert_eq!(decision.target_nodes(), vec!["proceed".to_string()]);
    }
}
