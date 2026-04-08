use naaf_core::budget::Services;
use naaf_core::errors::{Error, Result};
use naaf_core::steps::{BoxedRouter, BoxedTransformer, BoxedValidator};
use naaf_core::workflow_registry::WorkflowRegistry;
use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::accept::AcceptStep;
use crate::classify_input::ClassifyInput;
use crate::normalize::NormalizeStep;
use crate::package_steps::{
    ApplySectionPatchStep, FindingsAggregatorStep, PackageLlmAcceptanceStep,
    PackageLlmConsistencyReviewStep, PackageLlmNormalizeStep, PackageLlmRiskReviewStep,
    PackageLlmScopeStep, PackageLlmSkeletonStep, PackageLlmTargetedRemediationStep,
    RemediationPlanRouter, RemediationPlannerStep, ReviewFindingsRouter, WorkflowOutcomeStep,
};
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

#[derive(Default, Deserialize)]
struct PackageNormalizeConfig {
    #[serde(default)]
    context_keys: Vec<String>,
}

#[derive(Deserialize)]
struct ReviewFindingsRouterConfig {
    accepted_route: String,
    remediation_route: String,
}

#[derive(Deserialize)]
struct RemediationPlanRouterConfig {
    remediate_route: String,
    escalation_route: String,
}

pub fn register_legacy_steps<S: Services + 'static>(registry: &mut WorkflowRegistry<S>) {
    registry.register_transformer("openspec.propose", |_| {
        Ok(BoxedTransformer::new(ProposeStep::<S>::new()))
    });
    registry.register_transformer("openspec.classify_input", |_| {
        Ok(BoxedTransformer::new(ClassifyInput::<S>::new()))
    });
    registry.register_transformer("openspec.normalize", |_| {
        Ok(BoxedTransformer::new(NormalizeStep::<S>::new()))
    });
    registry.register_transformer("openspec.scope", |_| {
        Ok(BoxedTransformer::new(ScopeStep::<S>::new()))
    });
    registry.register_transformer("openspec.plan", |_| {
        Ok(BoxedTransformer::new(PlanStep::<S>::new()))
    });
    registry.register_transformer("openspec.accept", |_| {
        Ok(BoxedTransformer::new(AcceptStep::<S>::new()))
    });
    registry.register_transformer("openspec.greeting_terminal", |config| {
        let config: GreetingTerminalConfig = parse_config("openspec.greeting_terminal", config)?;
        Ok(BoxedTransformer::new(GreetingTerminal::<S>::new(
            config.response,
        )))
    });
    registry.register_transformer("openspec.escalation_terminal", |config| {
        let config: EscalationTerminalConfig =
            parse_config("openspec.escalation_terminal", config)?;
        Ok(BoxedTransformer::new(EscalationTerminal::<S>::new(
            config.message,
        )))
    });
    registry.register_router("openspec.input_classification_router", |config| {
        let config: InputClassificationRouterConfig =
            parse_config("openspec.input_classification_router", config)?;
        Ok(BoxedRouter::new(InputClassificationRouter::<S>::new(
            config.greeting_route,
            config.clarification_route,
            config.actionable_route,
        )))
    });
    registry.register_validator("openspec.done_validator", |_| {
        Ok(BoxedValidator::new(DoneValidator::<S>::new()))
    });
}

pub fn register_workflow_steps<S: Services + 'static>(registry: &mut WorkflowRegistry<S>) {
    registry.register_transformer("openspec.propose", |_| {
        Ok(BoxedTransformer::new(ProposeStep::<S>::new()))
    });
    registry.register_transformer("openspec.classify_input", |_| {
        Ok(BoxedTransformer::new(ClassifyInput::<S>::new()))
    });
    registry.register_transformer("openspec.normalize", |_| {
        Ok(BoxedTransformer::new(NormalizeStep::<S>::new()))
    });
    registry.register_transformer("openspec.scope", |_| {
        Ok(BoxedTransformer::new(ScopeStep::<S>::new()))
    });
    registry.register_transformer("openspec.plan", |_| {
        Ok(BoxedTransformer::new(PlanStep::<S>::new()))
    });
    registry.register_transformer("openspec.accept", |_| {
        Ok(BoxedTransformer::new(AcceptStep::<S>::new()))
    });
    registry.register_transformer("openspec.greeting_terminal", |config| {
        let config: GreetingTerminalConfig = parse_config("openspec.greeting_terminal", config)?;
        Ok(BoxedTransformer::new(GreetingTerminal::<S>::new(
            config.response,
        )))
    });
    registry.register_transformer("openspec.escalation_terminal", |config| {
        let config: EscalationTerminalConfig =
            parse_config("openspec.escalation_terminal", config)?;
        Ok(BoxedTransformer::new(EscalationTerminal::<S>::new(
            config.message,
        )))
    });
    registry.register_router("openspec.input_classification_router", |config| {
        let config: InputClassificationRouterConfig =
            parse_config("openspec.input_classification_router", config)?;
        Ok(BoxedRouter::new(InputClassificationRouter::<S>::new(
            config.greeting_route,
            config.clarification_route,
            config.actionable_route,
        )))
    });
    registry.register_validator("openspec.done_validator", |_| {
        Ok(BoxedValidator::new(DoneValidator::<S>::new()))
    });
    registry.register_transformer("openspec.package_llm_normalize", |config| {
        let config = if config.is_null() {
            PackageNormalizeConfig::default()
        } else {
            parse_config("openspec.package_llm_normalize", config)?
        };
        Ok(BoxedTransformer::new(
            PackageLlmNormalizeStep::<S>::new().with_context_keys(config.context_keys),
        ))
    });
    registry.register_transformer("openspec.package_llm_scope", |_| {
        Ok(BoxedTransformer::new(PackageLlmScopeStep::<S>::new()))
    });
    registry.register_transformer("openspec.package_llm_skeleton", |_| {
        Ok(BoxedTransformer::new(PackageLlmSkeletonStep::<S>::new()))
    });
    registry.register_transformer("openspec.package_llm_risk_review", |_| {
        Ok(BoxedTransformer::new(PackageLlmRiskReviewStep::<S>::new()))
    });
    registry.register_transformer("openspec.package_llm_consistency_review", |_| {
        Ok(BoxedTransformer::new(
            PackageLlmConsistencyReviewStep::<S>::new(),
        ))
    });
    registry.register_transformer("openspec.package_findings_aggregator", |_| {
        Ok(BoxedTransformer::new(FindingsAggregatorStep::<S>::new()))
    });
    registry.register_router("openspec.package_review_findings_router", |config| {
        let config: ReviewFindingsRouterConfig =
            parse_config("openspec.package_review_findings_router", config)?;
        Ok(BoxedRouter::new(ReviewFindingsRouter::<S>::new(
            config.accepted_route,
            config.remediation_route,
        )))
    });
    registry.register_transformer("openspec.package_remediation_planner", |_| {
        Ok(BoxedTransformer::new(RemediationPlannerStep::<S>::new()))
    });
    registry.register_router("openspec.package_remediation_plan_router", |config| {
        let config: RemediationPlanRouterConfig =
            parse_config("openspec.package_remediation_plan_router", config)?;
        Ok(BoxedRouter::new(RemediationPlanRouter::<S>::new(
            config.remediate_route,
            config.escalation_route,
        )))
    });
    registry.register_transformer("openspec.package_llm_targeted_remediation", |_| {
        Ok(BoxedTransformer::new(
            PackageLlmTargetedRemediationStep::<S>::new(),
        ))
    });
    registry.register_transformer("openspec.package_apply_section_patch", |_| {
        Ok(BoxedTransformer::new(ApplySectionPatchStep::<S>::new()))
    });
    registry.register_transformer("openspec.package_workflow_outcome", |_| {
        Ok(BoxedTransformer::new(WorkflowOutcomeStep::<S>::new()))
    });
    registry.register_transformer("openspec.package_llm_acceptance", |_| {
        Ok(BoxedTransformer::new(PackageLlmAcceptanceStep::<S>::new()))
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
    use crate::test_services::NoopServices;
    use naaf_core::workflow_loader::build_workflow;
    use naaf_core::workflow_package::{WorkflowNodeKind, WorkflowPackageNode};

    #[test]
    fn registers_openspec_steps() {
        let mut registry = WorkflowRegistry::<NoopServices>::new();
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

    #[test]
    fn registers_package_steps() {
        let mut registry = WorkflowRegistry::<NoopServices>::new();
        register_workflow_steps(&mut registry);

        let node = WorkflowPackageNode {
            id: "normalize".to_string(),
            kind: WorkflowNodeKind::Transformer,
            step: "openspec.package_llm_normalize".to_string(),
            config: serde_json::json!({ "context_keys": ["repository_context"] }),
        };

        let built = registry.build_node(&node).unwrap();
        assert_eq!(built.id(), "normalize");
    }

    #[test]
    fn builds_packaged_workflow_manifest() {
        let manifest_path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("openspec-package.toml");
        let package =
            naaf_core::workflow_package::WorkflowPackage::from_path(&manifest_path).unwrap();

        let mut registry = WorkflowRegistry::<NoopServices>::new();
        register_workflow_steps(&mut registry);

        let workflow = build_workflow(&package, &registry).unwrap();
        assert_eq!(workflow.name, "openspec-package");
    }
}
