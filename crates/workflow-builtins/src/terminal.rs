use workflow_core::budget::{DummyServices, ExecCtx};
use workflow_core::errors::StepError;
use workflow_core::steps::Transformer;
use workflow_schema::adapters::{get_typed, put_typed};
use workflow_schema::artifacts::ArtifactKey;
use workflow_schema::state::StateEnvelope;

use crate::classify_input::{Classification, InputClass};

/// Terminal handler for greeting inputs.
///
/// Produces a response for inputs that have been classified as greetings.
/// Validates that the input is actually a greeting before responding.
pub struct GreetingTerminal {
    classification_key: ArtifactKey,
    response_key: ArtifactKey,
    response: String,
}

impl GreetingTerminal {
    /// Creates a new GreetingTerminal with default artifact keys.
    ///
    /// Uses "classification" as the input key and "response" as the output key.
    pub fn new(response: impl Into<String>) -> Self {
        Self {
            classification_key: ArtifactKey::new("classification"),
            response_key: ArtifactKey::new("response"),
            response: response.into(),
        }
    }

    /// Creates a new GreetingTerminal with custom artifact keys.
    pub fn with_keys(
        classification_key: impl Into<String>,
        response_key: impl Into<String>,
        response: impl Into<String>,
    ) -> Self {
        Self {
            classification_key: ArtifactKey::new(classification_key),
            response_key: ArtifactKey::new(response_key),
            response: response.into(),
        }
    }
}

impl Transformer for GreetingTerminal {
    type Services = DummyServices;

    fn name(&self) -> &'static str {
        "greeting_terminal"
    }

    fn transform(
        &self,
        _ctx: &mut ExecCtx<Self::Services>,
        mut state: StateEnvelope,
    ) -> Result<StateEnvelope, StepError> {
        let classification: Classification =
            get_typed(&self.classification_key, &state).map_err(|e| {
                StepError::transformer(
                    "greeting_terminal",
                    format!(
                        "Failed to get classification from artifact key '{}': {}",
                        self.classification_key, e
                    ),
                )
            })?;

        if classification.class != InputClass::Greeting {
            return Err(StepError::transformer(
                "greeting_terminal",
                format!(
                    "Expected Greeting classification but got {:?} (confidence: {:.2})",
                    classification.class, classification.confidence
                ),
            ));
        }

        put_typed(self.response_key.clone(), self.response.clone(), &mut state);

        Ok(state)
    }
}

/// Terminal handler for escalated inputs requiring human attention.
///
/// Captures escalation metadata including the classification, confidence,
/// and a custom message for human review.
pub struct EscalationTerminal {
    classification_key: ArtifactKey,
    escalation_key: ArtifactKey,
    message: String,
}

impl EscalationTerminal {
    /// Creates a new EscalationTerminal with default artifact keys.
    ///
    /// Uses "classification" as the input key and "escalation" as the output key.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            classification_key: ArtifactKey::new("classification"),
            escalation_key: ArtifactKey::new("escalation"),
            message: message.into(),
        }
    }

    /// Creates a new EscalationTerminal with custom artifact keys.
    pub fn with_keys(
        classification_key: impl Into<String>,
        escalation_key: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            classification_key: ArtifactKey::new(classification_key),
            escalation_key: ArtifactKey::new(escalation_key),
            message: message.into(),
        }
    }
}

impl Transformer for EscalationTerminal {
    type Services = DummyServices;

    fn name(&self) -> &'static str {
        "escalation_terminal"
    }

    fn transform(
        &self,
        _ctx: &mut ExecCtx<Self::Services>,
        mut state: StateEnvelope,
    ) -> Result<StateEnvelope, StepError> {
        let classification: Classification =
            get_typed(&self.classification_key, &state).map_err(|e| {
                StepError::transformer(
                    "escalation_terminal",
                    format!(
                        "Failed to get classification from artifact key '{}': {}",
                        self.classification_key, e
                    ),
                )
            })?;

        let escalation_data = serde_json::json!({
            "message": self.message,
            "classification": classification.class,
            "confidence": classification.confidence,
        });

        put_typed(self.escalation_key.clone(), escalation_data, &mut state);

        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classify_input::InputClass;
    use workflow_core::budget::DummyServices;
    use workflow_schema::artifacts::ArtifactValue;
    use workflow_schema::execution_status::ExecutionStatus;
    use workflow_schema::lineage::Lineage;
    use workflow_schema::state::{RunId, StateEnvelope, StateId};
    use workflow_schema::state_kind::StateKind;

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

    fn make_ctx() -> ExecCtx<DummyServices> {
        ExecCtx::new(RunId::new(), DummyServices)
    }

    #[test]
    fn test_greeting_terminal_sets_response() {
        let terminal = GreetingTerminal::new("Hello! How can I help you today?");
        let mut ctx = make_ctx();
        let state = make_state_with_classification(InputClass::Greeting, 0.95);

        let result = terminal.transform(&mut ctx, state).unwrap();
        let response: String = get_typed(&ArtifactKey::new("response"), &result).unwrap();
        assert_eq!(response, "Hello! How can I help you today?");
    }

    #[test]
    fn test_greeting_terminal_custom_keys() {
        let terminal = GreetingTerminal::with_keys("class", "reply", "Good morning!");
        let mut state = StateEnvelope::new(
            StateId::new(),
            RunId::new(),
            StateKind::Proposed,
            Lineage::new(None, None, ExecutionStatus::Pending),
        );
        let classification = Classification {
            class: InputClass::Greeting,
            confidence: 0.88,
        };
        state.artifacts.insert(
            ArtifactKey::new("class"),
            ArtifactValue::json(serde_json::json!(classification)),
        );

        let mut ctx = make_ctx();
        let result = terminal.transform(&mut ctx, state).unwrap();
        let response: String = get_typed(&ArtifactKey::new("reply"), &result).unwrap();
        assert_eq!(response, "Good morning!");
    }

    #[test]
    fn test_escalation_terminal_sets_escalation_data() {
        let terminal = EscalationTerminal::new("This request needs human attention");
        let mut ctx = make_ctx();
        let state = make_state_with_classification(InputClass::Ambiguous, 0.65);

        let result = terminal.transform(&mut ctx, state).unwrap();
        let escalation: serde_json::Value =
            get_typed(&ArtifactKey::new("escalation"), &result).unwrap();

        assert_eq!(
            escalation.get("message").and_then(|m| m.as_str()),
            Some("This request needs human attention")
        );
        assert_eq!(
            escalation.get("classification").and_then(|c| c.as_str()),
            Some("Ambiguous")
        );
        assert_eq!(
            escalation.get("confidence").and_then(|c| c.as_f64()),
            Some(0.65)
        );
    }

    #[test]
    fn test_escalation_terminal_custom_keys() {
        let terminal = EscalationTerminal::with_keys("class", "escalate", "Needs review");
        let mut state = StateEnvelope::new(
            StateId::new(),
            RunId::new(),
            StateKind::Proposed,
            Lineage::new(None, None, ExecutionStatus::Pending),
        );
        let classification = Classification {
            class: InputClass::Actionable,
            confidence: 0.5,
        };
        state.artifacts.insert(
            ArtifactKey::new("class"),
            ArtifactValue::json(serde_json::json!(classification)),
        );

        let mut ctx = make_ctx();
        let result = terminal.transform(&mut ctx, state).unwrap();
        let escalation: serde_json::Value =
            get_typed(&ArtifactKey::new("escalate"), &result).unwrap();

        assert_eq!(
            escalation.get("message").and_then(|m| m.as_str()),
            Some("Needs review")
        );
    }

    #[test]
    fn test_greeting_terminal_rejects_non_greeting() {
        let terminal = GreetingTerminal::new("Hello!");
        let mut ctx = make_ctx();
        let state = make_state_with_classification(InputClass::Actionable, 0.9);

        let result = terminal.transform(&mut ctx, state);
        assert!(result.is_err());
    }

    #[test]
    fn test_greeting_terminal_rejects_ambiguous() {
        let terminal = GreetingTerminal::new("Hello!");
        let mut ctx = make_ctx();
        let state = make_state_with_classification(InputClass::Ambiguous, 0.6);

        let result = terminal.transform(&mut ctx, state);
        assert!(result.is_err());
    }

    #[test]
    fn test_greeting_terminal_missing_classification() {
        let terminal = GreetingTerminal::new("Hello!");
        let mut ctx = make_ctx();
        let state = StateEnvelope::new(
            StateId::new(),
            RunId::new(),
            StateKind::Proposed,
            Lineage::new(None, None, ExecutionStatus::Pending),
        );

        let result = terminal.transform(&mut ctx, state);
        assert!(result.is_err());
    }

    #[test]
    fn test_escalation_terminal_missing_classification() {
        let terminal = EscalationTerminal::new("Needs attention");
        let mut ctx = make_ctx();
        let state = StateEnvelope::new(
            StateId::new(),
            RunId::new(),
            StateKind::Proposed,
            Lineage::new(None, None, ExecutionStatus::Pending),
        );

        let result = terminal.transform(&mut ctx, state);
        assert!(result.is_err());
    }
}
