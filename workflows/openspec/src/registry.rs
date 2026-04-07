use naaf_core::budget::DummyServices;
use naaf_core::errors::{Error, Result};
use naaf_core::steps::{BoxedRouter, BoxedTransformer, BoxedValidator};
use naaf_core::workflow_registry::WorkflowRegistry;
use serde::de::DeserializeOwned;
use serde::Deserialize;

use crate::accept::AcceptStep;
use crate::classify_input::ClassifyInput;
use crate::normalize::NormalizeStep;
use crate::plan::PlanStep;
use crate::propose::ProposeStep;
use crate::routers::InputClassificationRouter;
use crate::scope::ScopeStep;
use crate::terminal::{EscalationTerminal, GreetingTerminal};
use crate::validators::DoneValidator;

#[derive(Deserialize)]
struct GreetingTerminalConfig {
    response: String,
}

#[derive(Deserialize)]
struct EscalationTerminalConfig {
    message: String,
}

#[derive(Deserialize)]
struct InputClassificationRouterConfig {
    greeting_route: String,
    clarification_route: String,
    actionable_route: String,
}

pub fn register_legacy_steps(registry: &mut WorkflowRegistry<DummyServices>) {
    registry.register_transformer("openspec.propose", |_| {
        Ok(BoxedTransformer::new(ProposeStep::new()))
    });
    registry.register_transformer("openspec.classify_input", |_| {
        Ok(BoxedTransformer::new(ClassifyInput::new()))
    });
    registry.register_transformer("openspec.normalize", |_| {
        Ok(BoxedTransformer::new(NormalizeStep::new()))
    });
    registry.register_transformer("openspec.scope", |_| {
        Ok(BoxedTransformer::new(ScopeStep::new()))
    });
    registry.register_transformer("openspec.plan", |_| {
        Ok(BoxedTransformer::new(PlanStep::new()))
    });
    registry.register_transformer("openspec.accept", |_| {
        Ok(BoxedTransformer::new(AcceptStep::new()))
    });
    registry.register_transformer("openspec.greeting_terminal", |config| {
        let config: GreetingTerminalConfig = parse_config("openspec.greeting_terminal", config)?;
        Ok(BoxedTransformer::new(GreetingTerminal::new(
            config.response,
        )))
    });
    registry.register_transformer("openspec.escalation_terminal", |config| {
        let config: EscalationTerminalConfig =
            parse_config("openspec.escalation_terminal", config)?;
        Ok(BoxedTransformer::new(EscalationTerminal::new(
            config.message,
        )))
    });
    registry.register_router("openspec.input_classification_router", |config| {
        let config: InputClassificationRouterConfig =
            parse_config("openspec.input_classification_router", config)?;
        Ok(BoxedRouter::new(InputClassificationRouter::new(
            config.greeting_route,
            config.clarification_route,
            config.actionable_route,
        )))
    });
    registry.register_validator("openspec.done_validator", |_| {
        Ok(BoxedValidator::new(DoneValidator::new()))
    });
}

pub fn register_workflow_steps(registry: &mut WorkflowRegistry<DummyServices>) {
    registry.register_transformer("openspec.propose", |_| {
        Ok(BoxedTransformer::new(ProposeStep::new()))
    });
    registry.register_transformer("openspec.classify_input", |_| {
        Ok(BoxedTransformer::new(ClassifyInput::new()))
    });
    registry.register_transformer("openspec.normalize", |_| {
        Ok(BoxedTransformer::new(NormalizeStep::new()))
    });
    registry.register_transformer("openspec.scope", |_| {
        Ok(BoxedTransformer::new(ScopeStep::new()))
    });
    registry.register_transformer("openspec.plan", |_| {
        Ok(BoxedTransformer::new(PlanStep::new()))
    });
    registry.register_transformer("openspec.accept", |_| {
        Ok(BoxedTransformer::new(AcceptStep::new()))
    });
    registry.register_transformer("openspec.greeting_terminal", |config| {
        let config: GreetingTerminalConfig = parse_config("openspec.greeting_terminal", config)?;
        Ok(BoxedTransformer::new(GreetingTerminal::new(
            config.response,
        )))
    });
    registry.register_transformer("openspec.escalation_terminal", |config| {
        let config: EscalationTerminalConfig =
            parse_config("openspec.escalation_terminal", config)?;
        Ok(BoxedTransformer::new(EscalationTerminal::new(
            config.message,
        )))
    });
    registry.register_router("openspec.input_classification_router", |config| {
        let config: InputClassificationRouterConfig =
            parse_config("openspec.input_classification_router", config)?;
        Ok(BoxedRouter::new(InputClassificationRouter::new(
            config.greeting_route,
            config.clarification_route,
            config.actionable_route,
        )))
    });
    registry.register_validator("openspec.done_validator", |_| {
        Ok(BoxedValidator::new(DoneValidator::new()))
    });
}

fn parse_config<T>(step: &str, config: &serde_json::Value) -> Result<T>
where
    T: DeserializeOwned,
{
    serde_json::from_value(config.clone())
        .map_err(|err| Error::WorkflowPackage(format!("Invalid config for step '{step}': {err}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use naaf_core::workflow_package::{WorkflowNodeKind, WorkflowPackageNode};

    #[test]
    fn registers_openspec_steps() {
        let mut registry = WorkflowRegistry::<DummyServices>::new();
        register_workflow_steps(&mut registry);

        let node = WorkflowPackageNode {
            id: "propose".to_string(),
            kind: WorkflowNodeKind::Transformer,
            step: "openspec.propose".to_string(),
            config: serde_json::Value::Null,
        };

        let built = registry.build_node(&node).unwrap();
        assert_eq!(built.id(), "propose");
    }
}
