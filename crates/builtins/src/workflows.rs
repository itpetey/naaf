//! Workflow definitions using the new runtime.
//!
//! This module provides ready-to-use workflow definitions.

use std::sync::Arc;

use naaf_core::budget::DummyServices;
use naaf_core::builder::WorkflowBuilder;
use naaf_core::errors::Result;
use naaf_core::graph::CompiledWorkflow;
use naaf_core::steps::{BoxedRouter, BoxedTransformer, BoxedValidator};
use naaf_llm::LlmServices;
use naaf_providers::ModelProvider;

use crate::accept::AcceptStep;
use crate::classify_input::ClassifyInput;
use crate::llm_steps::{LlmAcceptanceStep, LlmNormalizeStep, LlmScopeStep, LlmSkeletonStep};
use crate::normalize::NormalizeStep;
use crate::plan::PlanStep;
use crate::propose::ProposeStep;
use crate::routers::InputClassificationRouter;
use crate::scope::ScopeStep;
use crate::terminal::{EscalationTerminal, GreetingTerminal};
use crate::validators::DoneValidator;

pub fn draft_request_workflow() -> Result<CompiledWorkflow<DummyServices>> {
    let propose_step: BoxedTransformer<DummyServices> = BoxedTransformer::new(ProposeStep::new());
    let classify_step: BoxedTransformer<DummyServices> =
        BoxedTransformer::new(ClassifyInput::new());
    let normalize_step: BoxedTransformer<DummyServices> =
        BoxedTransformer::new(NormalizeStep::new());
    let scope_step: BoxedTransformer<DummyServices> = BoxedTransformer::new(ScopeStep::new());
    let plan_step: BoxedTransformer<DummyServices> = BoxedTransformer::new(PlanStep::new());
    let accept_step: BoxedTransformer<DummyServices> = BoxedTransformer::new(AcceptStep::new());

    let greeting_terminal: BoxedTransformer<DummyServices> =
        BoxedTransformer::new(GreetingTerminal::new("Hello! How can I help you today?"));
    let clarification_terminal: BoxedTransformer<DummyServices> =
        BoxedTransformer::new(EscalationTerminal::new("This request needs clarification"));

    let input_router: BoxedRouter<DummyServices> = BoxedRouter::new(
        InputClassificationRouter::new("greeting_terminal", "clarification_terminal", "normalize"),
    );

    let done_validator1: BoxedValidator<DummyServices> = BoxedValidator::new(DoneValidator::new());
    let done_validator2: BoxedValidator<DummyServices> = BoxedValidator::new(DoneValidator::new());
    let done_validator3: BoxedValidator<DummyServices> = BoxedValidator::new(DoneValidator::new());

    let workflow = WorkflowBuilder::new("draft_request")
        .step("propose", propose_step)
        .step("classify_input", classify_step)
        .route("initial_decision", input_router)
        .step("greeting_terminal", greeting_terminal)
        .step("clarification_terminal", clarification_terminal)
        .step("normalize", normalize_step)
        .step("scope", scope_step)
        .step("plan", plan_step)
        .step("accept", accept_step)
        .terminal("greeting_done", done_validator1)
        .terminal("clarification_done", done_validator2)
        .terminal("done", done_validator3)
        .path("propose", "classify_input")
        .path("classify_input", "initial_decision")
        .path("initial_decision", "greeting_terminal")
        .path("initial_decision", "clarification_terminal")
        .path("initial_decision", "normalize")
        .path("greeting_terminal", "greeting_done")
        .path("clarification_terminal", "clarification_done")
        .path("normalize", "scope")
        .path("scope", "plan")
        .path("plan", "accept")
        .path("accept", "done")
        .compile()?;

    Ok(workflow)
}

/// Creates an LLM-powered workflow for processing normalized specifications.
///
/// This workflow uses LLM services to:
/// 1. Normalize the request into a structured spec
/// 2. Analyze scope of the work
/// 3. Build a proposal skeleton
/// 4. Generate acceptance criteria
///
/// # Arguments
///
/// * `provider` - The model provider (e.g., OpenAI, Anthropic)
/// * `model` - The model name to use
///
/// # Returns
///
/// A compiled workflow ready for execution with LlmServices
///
/// # Panics
///
/// Panics if called outside of a Tokio runtime context.
pub fn openspec_happy_path_llm<P: ModelProvider + Send + Sync + 'static>(
    provider: Arc<P>,
    model: String,
) -> Result<CompiledWorkflow<LlmServices<P>>> {
    let handle = tokio::runtime::Handle::current();
    let services = LlmServices::new(provider, model);

    let normalize_step: BoxedTransformer<LlmServices<P>> =
        BoxedTransformer::new(LlmNormalizeStep::new(services.clone(), handle.clone()));
    let scope_step: BoxedTransformer<LlmServices<P>> =
        BoxedTransformer::new(LlmScopeStep::new(services.clone(), handle.clone()));
    let skeleton_step: BoxedTransformer<LlmServices<P>> =
        BoxedTransformer::new(LlmSkeletonStep::new(services.clone(), handle.clone()));
    let acceptance_step: BoxedTransformer<LlmServices<P>> =
        BoxedTransformer::new(LlmAcceptanceStep::new(services, handle));

    let workflow = WorkflowBuilder::new("openspec-happy-path-llm")
        .step("normalize", normalize_step)
        .step("scope", scope_step)
        .step("skeleton", skeleton_step)
        .step("acceptance", acceptance_step)
        .path("normalize", "scope")
        .path("scope", "skeleton")
        .path("skeleton", "acceptance")
        .compile()?;

    Ok(workflow)
}

/// Creates an LLM-powered workflow with mock services for testing.
///
/// This is intended for testing and demonstration purposes only.
/// Must be called from within a Tokio runtime context.
#[cfg(test)]
pub fn openspec_happy_path_mock(
    mock_services: crate::mock_llm::MockLlmServices,
) -> Result<CompiledWorkflow<crate::mock_llm::MockLlmServices>> {
    let handle = tokio::runtime::Handle::current();

    let normalize_step: BoxedTransformer<crate::mock_llm::MockLlmServices> =
        BoxedTransformer::new(LlmNormalizeStep::new(mock_services.clone(), handle.clone()));
    let scope_step: BoxedTransformer<crate::mock_llm::MockLlmServices> =
        BoxedTransformer::new(LlmScopeStep::new(mock_services.clone(), handle.clone()));
    let skeleton_step: BoxedTransformer<crate::mock_llm::MockLlmServices> =
        BoxedTransformer::new(LlmSkeletonStep::new(mock_services.clone(), handle.clone()));
    let acceptance_step: BoxedTransformer<crate::mock_llm::MockLlmServices> =
        BoxedTransformer::new(LlmAcceptanceStep::new(mock_services, handle));

    let workflow = WorkflowBuilder::new("openspec-happy-path-mock")
        .step("normalize", normalize_step)
        .step("scope", scope_step)
        .step("skeleton", skeleton_step)
        .step("acceptance", acceptance_step)
        .path("normalize", "scope")
        .path("scope", "skeleton")
        .path("skeleton", "acceptance")
        .compile()?;

    Ok(workflow)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock_llm::MockLlmServices;
    use naaf_core::budget::{DummyServices, ExecCtx};
    use naaf_core::executor::Executor;
    use naaf_schema::artifacts::{ArtifactKey, ArtifactValue};
    use naaf_schema::execution_status::ExecutionStatus;
    use naaf_schema::lineage::Lineage;
    use naaf_schema::state::{RunId, StateEnvelope, StateId};
    use naaf_schema::state_kind::StateKind;
    use naaf_schema::{AcceptanceCriteriaSet, NormalizedSpec, ProposalSkeleton, ScopeReport};

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

    fn make_ctx() -> ExecCtx<DummyServices> {
        ExecCtx::new(RunId::new(), DummyServices)
    }

    #[test]
    fn test_draft_request_workflow_compiles() {
        let workflow = draft_request_workflow();
        if workflow.is_err() {
            let err = workflow.as_ref().err().unwrap();
            eprintln!("Workflow compilation error: {}", err);
        }
        assert!(workflow.is_ok());
        let workflow = workflow.unwrap();
        assert_eq!(workflow.name, "draft_request");
        println!("Workflow entry point: {}", workflow.entry_point);
        println!(
            "Workflow nodes: {:?}",
            workflow.nodes.iter().map(|n| n.id()).collect::<Vec<_>>()
        );
        println!(
            "Workflow edges: {:?}",
            workflow
                .edges
                .iter()
                .map(|e| (e.source.as_str(), e.target.as_str()))
                .collect::<Vec<_>>()
        );
    }

    #[tokio::test]
    async fn test_greeting_input() {
        let workflow = draft_request_workflow().unwrap();
        let executor = Executor::new(workflow).unwrap();
        let mut ctx = make_ctx();
        let state = make_state_with_input("Hi");

        let result = executor.execute(&mut ctx, state).await;
        if result.is_err() {
            let err = result.as_ref().err().unwrap();
            eprintln!("Error: {}", err);
        }
        assert!(result.is_ok());

        let final_state = result.unwrap();
        assert!(
            final_state
                .artifacts
                .contains_key(&ArtifactKey::new("response"))
        );
        let response: String =
            naaf_schema::adapters::get_typed(&ArtifactKey::new("response"), &final_state)
                .expect("Should have response artifact");
        assert_eq!(response, "Hello! How can I help you today?");
    }

    #[tokio::test]
    async fn test_ambiguous_input() {
        let workflow = draft_request_workflow().unwrap();
        let executor = Executor::new(workflow).unwrap();
        let mut ctx = make_ctx();
        let state = make_state_with_input("Could you help?");

        let result = executor.execute(&mut ctx, state).await;
        assert!(result.is_ok());

        let final_state = result.unwrap();
        assert!(
            final_state
                .artifacts
                .contains_key(&ArtifactKey::new("escalation"))
        );
        let escalation: serde_json::Value =
            naaf_schema::adapters::get_typed(&ArtifactKey::new("escalation"), &final_state)
                .expect("Should have escalation artifact");
        assert_eq!(
            escalation.get("message").and_then(|m| m.as_str()),
            Some("This request needs clarification")
        );
    }

    #[tokio::test]
    async fn test_actionable_input() {
        let workflow = draft_request_workflow().unwrap();
        let executor = Executor::new(workflow).unwrap();
        let mut ctx = make_ctx();
        let state = make_state_with_input("Create a file");

        let result = executor.execute(&mut ctx, state).await;
        assert!(result.is_ok());

        let final_state = result.unwrap();
        assert!(
            final_state
                .artifacts
                .contains_key(&ArtifactKey::new("acceptance"))
        );
        let acceptance: crate::accept::Acceptance =
            naaf_schema::adapters::get_typed(&ArtifactKey::new("acceptance"), &final_state)
                .expect("Should have acceptance artifact");
        assert!(acceptance.accepted);
    }

    #[tokio::test]
    async fn test_unicode_input() {
        let workflow = draft_request_workflow().unwrap();
        let executor = Executor::new(workflow).unwrap();
        let mut ctx = make_ctx();
        let state = make_state_with_input("你好世界");

        let result = executor.execute(&mut ctx, state).await;
        assert!(result.is_ok());

        let final_state = result.unwrap();
        // Unicode input should be classified as ambiguous (no clear action verbs)
        assert!(
            final_state
                .artifacts
                .contains_key(&ArtifactKey::new("escalation"))
        );
    }

    #[tokio::test]
    async fn test_empty_input() {
        let workflow = draft_request_workflow().unwrap();
        let executor = Executor::new(workflow).unwrap();
        let mut ctx = make_ctx();
        let state = make_state_with_input("");

        let result = executor.execute(&mut ctx, state).await;
        // Empty input should classify as Ambiguous and route to clarification
        assert!(result.is_ok());
        let final_state = result.unwrap();
        assert!(
            final_state
                .artifacts
                .contains_key(&ArtifactKey::new("escalation"))
        );
    }

    #[tokio::test]
    async fn test_whitespace_only_input() {
        let workflow = draft_request_workflow().unwrap();
        let executor = Executor::new(workflow).unwrap();
        let mut ctx = make_ctx();
        let state = make_state_with_input("   \t\n   ");

        let result = executor.execute(&mut ctx, state).await;
        assert!(result.is_ok());
        let final_state = result.unwrap();
        assert!(
            final_state
                .artifacts
                .contains_key(&ArtifactKey::new("escalation"))
        );
    }

    #[tokio::test]
    async fn test_long_input() {
        let workflow = draft_request_workflow().unwrap();
        let executor = Executor::new(workflow).unwrap();
        let mut ctx = make_ctx();
        let long_input = "Create a file ".repeat(1000);
        let state = make_state_with_input(&long_input);

        let result = executor.execute(&mut ctx, state).await;
        assert!(result.is_ok());

        let final_state = result.unwrap();
        assert!(
            final_state
                .artifacts
                .contains_key(&ArtifactKey::new("acceptance"))
        );
    }

    #[tokio::test]
    async fn test_mixed_case_action_verbs() {
        let workflow = draft_request_workflow().unwrap();
        let executor = Executor::new(workflow).unwrap();
        let mut ctx = make_ctx();
        let state = make_state_with_input("IMPLEMENT a new feature");

        let result = executor.execute(&mut ctx, state).await;
        assert!(result.is_ok());

        let final_state = result.unwrap();
        assert!(
            final_state
                .artifacts
                .contains_key(&ArtifactKey::new("acceptance"))
        );
        let acceptance: crate::accept::Acceptance =
            naaf_schema::adapters::get_typed(&ArtifactKey::new("acceptance"), &final_state)
                .expect("Should have acceptance artifact");
        assert!(acceptance.accepted);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_openspec_happy_path_mock_workflow() {
        let mock_services = MockLlmServices::new(MockLlmServices::default_sequence());
        let workflow = openspec_happy_path_mock(mock_services).unwrap();
        let executor = Executor::new(workflow).unwrap();
        let mut ctx = ExecCtx::new(RunId::new(), MockLlmServices::new(vec![]));

        let mut state = StateEnvelope::new(
            StateId::new(),
            RunId::new(),
            StateKind::Proposed,
            Lineage::new(None, None, ExecutionStatus::Pending),
        );
        state.artifacts.insert(
            ArtifactKey::new("input"),
            ArtifactValue::text("Add user authentication"),
        );

        let result = executor.execute(&mut ctx, state).await;
        assert!(
            result.is_ok(),
            "Workflow execution failed: {:?}",
            result.err()
        );

        let final_state = result.unwrap();

        // Verify normalized_spec artifact
        assert!(
            final_state
                .artifacts
                .contains_key(&ArtifactKey::new("normalized_spec")),
            "Missing normalized_spec artifact"
        );
        let normalized_spec: NormalizedSpec =
            naaf_schema::adapters::get_typed(&ArtifactKey::new("normalized_spec"), &final_state)
                .expect("Should have normalized_spec artifact");
        assert!(!normalized_spec.problem_statement.is_empty());

        // Verify scope_report artifact
        assert!(
            final_state
                .artifacts
                .contains_key(&ArtifactKey::new("scope_report")),
            "Missing scope_report artifact"
        );
        let scope_report: ScopeReport =
            naaf_schema::adapters::get_typed(&ArtifactKey::new("scope_report"), &final_state)
                .expect("Should have scope_report artifact");
        assert!(!scope_report.in_scope_items.is_empty());

        // Verify proposal_skeleton artifact
        assert!(
            final_state
                .artifacts
                .contains_key(&ArtifactKey::new("proposal_skeleton")),
            "Missing proposal_skeleton artifact"
        );
        let proposal: ProposalSkeleton =
            naaf_schema::adapters::get_typed(&ArtifactKey::new("proposal_skeleton"), &final_state)
                .expect("Should have proposal_skeleton artifact");
        assert!(!proposal.title.is_empty());

        // Verify acceptance_criteria artifact
        assert!(
            final_state
                .artifacts
                .contains_key(&ArtifactKey::new("acceptance_criteria")),
            "Missing acceptance_criteria artifact"
        );
        let criteria: AcceptanceCriteriaSet = naaf_schema::adapters::get_typed(
            &ArtifactKey::new("acceptance_criteria"),
            &final_state,
        )
        .expect("Should have acceptance_criteria artifact");
        assert!(!criteria.criteria.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_openspec_workflow_artifact_chain() {
        let mock_services = MockLlmServices::new(MockLlmServices::default_sequence());
        let workflow = openspec_happy_path_mock(mock_services).unwrap();
        let executor = Executor::new(workflow).unwrap();
        let mut ctx = ExecCtx::new(RunId::new(), MockLlmServices::new(vec![]));

        let mut state = StateEnvelope::new(
            StateId::new(),
            RunId::new(),
            StateKind::Proposed,
            Lineage::new(None, None, ExecutionStatus::Pending),
        );
        state.artifacts.insert(
            ArtifactKey::new("input"),
            ArtifactValue::text("Test request"),
        );

        let result = executor.execute(&mut ctx, state).await;
        assert!(result.is_ok());

        let final_state = result.unwrap();

        // Verify that all expected artifacts are present and properly typed
        let artifacts = &final_state.artifacts;
        assert!(artifacts.contains_key(&ArtifactKey::new("normalized_spec")));
        assert!(artifacts.contains_key(&ArtifactKey::new("scope_report")));
        assert!(artifacts.contains_key(&ArtifactKey::new("proposal_skeleton")));
        assert!(artifacts.contains_key(&ArtifactKey::new("acceptance_criteria")));

        // Verify artifact count
        assert_eq!(
            artifacts.len(),
            5,
            "Expected input plus 4 derived artifacts in final state"
        );
    }
}
