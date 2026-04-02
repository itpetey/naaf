//! Workflow definitions using the new runtime.
//!
//! This module provides ready-to-use workflow definitions.

use workflow_core::budget::DummyServices;
use workflow_core::builder::WorkflowBuilder;
use workflow_core::errors::Result;
use workflow_core::graph::CompiledWorkflow;
use workflow_core::steps::{BoxedRouter, BoxedTransformer, BoxedValidator};

use crate::accept::AcceptStep;
use crate::classify_input::ClassifyInput;
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

#[cfg(test)]
mod tests {
    use super::*;
    use workflow_core::budget::{DummyServices, ExecCtx};
    use workflow_core::executor::Executor;
    use workflow_schema::artifacts::{ArtifactKey, ArtifactValue};
    use workflow_schema::execution_status::ExecutionStatus;
    use workflow_schema::lineage::Lineage;
    use workflow_schema::state::{RunId, StateEnvelope, StateId};
    use workflow_schema::state_kind::StateKind;

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
            workflow_schema::adapters::get_typed(&ArtifactKey::new("response"), &final_state)
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
            workflow_schema::adapters::get_typed(&ArtifactKey::new("escalation"), &final_state)
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
            workflow_schema::adapters::get_typed(&ArtifactKey::new("acceptance"), &final_state)
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
            workflow_schema::adapters::get_typed(&ArtifactKey::new("acceptance"), &final_state)
                .expect("Should have acceptance artifact");
        assert!(acceptance.accepted);
    }
}
