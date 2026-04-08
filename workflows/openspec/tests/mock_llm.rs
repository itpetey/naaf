//! Mock LLM services for testing workflow steps.

use naaf_core::budget::Services;
use naaf_core::errors::StepError;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use naaf_openspec::{
    AcceptanceCriteriaSet, Criterion, NormalizedSpec, ProposalSkeleton, ScopeReport,
};

#[derive(Debug)]
pub struct MockLlmServices {
    responses: Vec<String>,
    call_count: Arc<AtomicUsize>,
}

impl Clone for MockLlmServices {
    fn clone(&self) -> Self {
        Self {
            responses: self.responses.clone(),
            call_count: Arc::clone(&self.call_count),
        }
    }
}

impl MockLlmServices {
    pub fn new(responses: Vec<String>) -> Self {
        Self {
            responses,
            call_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn normalized_spec_response() -> String {
        let spec = NormalizedSpec {
            problem_statement: "Add user authentication".to_string(),
            desired_outcome: "Secure login system".to_string(),
            explicit_constraints: vec!["Must use JWT".to_string()],
            implied_constraints: vec![],
            non_goals: vec!["UI redesign".to_string()],
            open_questions: vec!["Token expiry time?".to_string()],
            ambiguity_flags: vec![],
            assumptions: vec!["HTTPS available".to_string()],
        };
        serde_json::to_string_pretty(&spec).unwrap()
    }

    pub fn scope_report_response() -> String {
        let report = ScopeReport {
            in_scope_items: vec!["Authentication endpoints".to_string()],
            out_of_scope_items: vec!["Password reset UI".to_string()],
            dependencies: vec!["Database".to_string()],
            rollout_assumptions: vec!["Deploy during off-peak".to_string()],
            risk_multipliers: vec![],
            inferred_scope_items: vec![],
        };
        serde_json::to_string_pretty(&report).unwrap()
    }

    pub fn proposal_skeleton_response() -> String {
        let skeleton = ProposalSkeleton {
            title: "JWT Authentication System".to_string(),
            summary: "Implement JWT-based authentication".to_string(),
            motivation: "Secure API access".to_string(),
            goals: vec![
                "Protect endpoints".to_string(),
                "Validate users".to_string(),
            ],
            non_goals: vec!["Social login".to_string()],
            proposed_design: "Use JWT tokens with 24-hour expiry".to_string(),
            alternatives_considered: "Session-based auth (rejected for scalability)".to_string(),
            risks: "Token theft, secret key management".to_string(),
            rollout_plan: "Deploy to staging first".to_string(),
            open_questions: vec!["Token refresh strategy?".to_string()],
            acceptance_criteria: vec!["Users can log in".to_string()],
            todo_markers: vec![],
        };
        serde_json::to_string_pretty(&skeleton).unwrap()
    }

    pub fn acceptance_criteria_response() -> String {
        let criteria = AcceptanceCriteriaSet {
            criteria: vec![
                Criterion {
                    id: "AC-1".to_string(),
                    statement: "Users can authenticate with valid credentials".to_string(),
                    traceability: vec!["Goal 1".to_string()],
                    measurability: "measurable".to_string(),
                },
                Criterion {
                    id: "AC-2".to_string(),
                    statement: "Invalid tokens are rejected".to_string(),
                    traceability: vec!["Goal 2".to_string()],
                    measurability: "measurable".to_string(),
                },
            ],
            gaps: vec![],
        };
        serde_json::to_string_pretty(&criteria).unwrap()
    }

    pub fn default_sequence() -> Vec<String> {
        vec![
            Self::normalized_spec_response(),
            Self::scope_report_response(),
            Self::proposal_skeleton_response(),
            Self::acceptance_criteria_response(),
        ]
    }

    pub fn call_count(&self) -> usize {
        self.call_count.load(Ordering::SeqCst)
    }
}

impl Services for MockLlmServices {
    type Error = StepError;

    async fn call(&self, _service: &str, _request: &[u8]) -> Result<Vec<u8>, Self::Error> {
        let count = self.call_count.load(Ordering::SeqCst);
        if count < self.responses.len() {
            let response = self.responses[count].clone();
            self.call_count.store(count + 1, Ordering::SeqCst);
            Ok(response.into_bytes())
        } else {
            Err(StepError::transformer(
                "mock_llm",
                "No more mock responses available",
            ))
        }
    }
}
