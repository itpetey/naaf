//! LLM-powered workflow steps using OpenSpec prompts.

use naaf_core::budget::Services;
use naaf_core::errors::StepError;
use naaf_core::steps::Transformer;
use naaf_schema::adapters::{get_typed, put_typed};
use naaf_schema::artifacts::ArtifactKey;
use naaf_schema::state::StateEnvelope;
use serde::de::DeserializeOwned;
use tokio::runtime::Handle;

use crate::llm_json::parse_json;
use crate::{AcceptanceCriteriaSet, NormalizedSpec, ProposalSkeleton, ScopeReport};

fn call_and_decode<T, S>(
    handle: &Handle,
    services: &S,
    step_name: &'static str,
    prompt: String,
) -> Result<T, StepError>
where
    T: DeserializeOwned,
    S: Services + Send + Sync,
{
    let response_bytes = tokio::task::block_in_place(|| {
        handle.block_on(async { services.call("llm", prompt.as_bytes()).await })
    })
    .map_err(|e| StepError::transformer(step_name, format!("LLM call failed: {}", e)))?;

    let response = String::from_utf8_lossy(&response_bytes);
    parse_json(step_name, &response)
}

const NORMALIZE_PROMPT: &str = r#"You are a proposal normalizer.

Input:
- A raw feature request or proposal seed.

Task:
Transform the request into a structured specification draft with these fields:
- problem_statement
- desired_outcome
- explicit_constraints
- implied_constraints
- non_goals
- open_questions
- ambiguity_flags
- assumptions

Rules:
- Do not invent product facts.
- If information is missing, record it under open_questions or ambiguity_flags.
- Preserve the original intent faithfully.
- Prefer concise, concrete language.

User Request:
{user_prompt}

Output:
Return valid JSON only using the required schema."#;

const SCOPE_PROMPT: &str = r#"You are a scope analyst.

Task:
Analyse the normalised specification and extract scope information.

Input:
{normalized_spec}

Extract:
- in_scope_items: What IS included in this request
- out_of_scope_items: What is explicitly NOT included
- dependencies: External systems or services this depends on
- rollout_assumptions: Assumptions about deployment environment
- risk_multipliers: Factors that could increase scope
- inferred_scope_items: Items inferred from context but not explicit

Rules:
- Separate explicit scope from inferred scope.
- Mark any inference as inferred.
- Do not propose solutions yet.
- Base findings only on the input specification.

Output:
Return valid JSON only using this schema:
{
  "in_scope_items": ["item1", "item2"],
  "out_of_scope_items": ["item1"],
  "dependencies": ["dep1"],
  "rollout_assumptions": ["assumption1"],
  "risk_multipliers": ["multiplier1"],
  "inferred_scope_items": ["inferred1"]
}"#;

const SKELETON_PROMPT: &str = r#"You are a proposal structurer.

Task:
Produce an OpenSpec proposal skeleton from the normalised specification and scope analysis.

Normalised Specification:
{normalized_spec}

Scope Analysis:
{scope_report}

Produce a proposal with these sections:
- Title: A clear, concise title
- Summary: One-line description
- Motivation: Why this change is needed
- Goals: Array of strings describing objectives
- Non-Goals: Array of strings describing what is NOT included
- Proposed Design: Detailed design description
- Alternatives Considered: Other approaches evaluated
- Risks: Potential issues and mitigations
- Rollout Plan: How to deploy this change
- Open Questions: Array of unresolved questions
- Acceptance Criteria: Array of acceptance criteria
- Todo Markers: Array of TODO items (use TODO(reason) format)

Rules:
- Use placeholders only where evidence is missing.
- Mark every placeholder with TODO(<reason>).
- Do not fabricate operational details.
- Base content on the input specification and scope.

Output:
Return valid JSON only using this schema:
{
  "title": "string",
  "summary": "string",
  "motivation": "string",
  "goals": ["goal1", "goal2"],
  "non_goals": ["non-goal1"],
  "proposed_design": "string",
  "alternatives_considered": "string",
  "risks": "string",
  "rollout_plan": "string",
  "open_questions": ["question1"],
  "acceptance_criteria": ["criteria1"],
  "todo_markers": ["TODO(reason)"]
}"#;

const ACCEPTANCE_PROMPT: &str = r#"You are an acceptance criteria author.

Task:
Write acceptance criteria for the proposal that are observable, testable, and traceable.

Normalised Specification:
{normalized_spec}

Proposal Skeleton:
{proposal_skeleton}

Write acceptance criteria that are:
- Observable: Can be verified by inspection or testing
- Testable: Can be validated with concrete tests
- Implementation-agnostic: Don't assume specific implementations
- Traceable: Connect back to stated goals

Rules:
- Each criterion must be atomic (one thing).
- Avoid vague terms like "fast", "robust", "user-friendly", or "works well".
- Where measurable thresholds are unknown, create explicit placeholders.
- Criteria should verify the solution meets its goals.

Output:
Return valid JSON only using this schema:
{
  "criteria": [
    {
      "id": "AC-1",
      "statement": "The system must...",
      "traceability": ["Goal-1"],
      "measurability": "measurable"
    }
  ],
  "gaps": ["Any unaddressed requirements"]
}"#;

pub struct LlmNormalizeStep<S: Services> {
    input_key: ArtifactKey,
    output_key: ArtifactKey,
    services: S,
    handle: Handle,
}

impl<S: Services> LlmNormalizeStep<S> {
    pub fn new(services: S, handle: Handle) -> Self {
        Self {
            input_key: ArtifactKey::new("input"),
            output_key: ArtifactKey::new("normalized_spec"),
            services,
            handle,
        }
    }

    pub fn with_keys(
        input_key: impl Into<String>,
        output_key: impl Into<String>,
        services: S,
        handle: Handle,
    ) -> Self {
        Self {
            input_key: ArtifactKey::new(input_key),
            output_key: ArtifactKey::new(output_key),
            services,
            handle,
        }
    }

    fn render_prompt(&self, input: &str) -> String {
        NORMALIZE_PROMPT.replace("{user_prompt}", input)
    }
}

impl<S: Services + Send + Sync> Transformer for LlmNormalizeStep<S> {
    type Services = S;

    fn name(&self) -> &'static str {
        "llm_normalize"
    }

    fn transform(
        &self,
        _ctx: &mut naaf_core::budget::ExecCtx<Self::Services>,
        mut state: StateEnvelope,
    ) -> Result<StateEnvelope, StepError> {
        let input: String = get_typed(&self.input_key, &state).map_err(|e| {
            StepError::transformer("llm_normalize", format!("Failed to get input: {}", e))
        })?;

        let prompt = self.render_prompt(&input);
        let spec: NormalizedSpec =
            call_and_decode(&self.handle, &self.services, self.name(), prompt)?;
        put_typed(self.output_key.clone(), spec, &mut state);
        Ok(state)
    }
}

pub struct LlmScopeStep<S: Services> {
    input_key: ArtifactKey,
    output_key: ArtifactKey,
    services: S,
    handle: Handle,
}

impl<S: Services> LlmScopeStep<S> {
    pub fn new(services: S, handle: Handle) -> Self {
        Self {
            input_key: ArtifactKey::new("normalized_spec"),
            output_key: ArtifactKey::new("scope_report"),
            services,
            handle,
        }
    }

    pub fn with_keys(
        input_key: impl Into<String>,
        output_key: impl Into<String>,
        services: S,
        handle: Handle,
    ) -> Self {
        Self {
            input_key: ArtifactKey::new(input_key),
            output_key: ArtifactKey::new(output_key),
            services,
            handle,
        }
    }

    fn render_prompt(&self, spec: &NormalizedSpec) -> String {
        let spec_json = serde_json::to_string_pretty(spec).unwrap_or_default();
        SCOPE_PROMPT.replace("{normalized_spec}", &spec_json)
    }
}

impl<S: Services + Send + Sync> Transformer for LlmScopeStep<S> {
    type Services = S;

    fn name(&self) -> &'static str {
        "llm_scope"
    }

    fn transform(
        &self,
        _ctx: &mut naaf_core::budget::ExecCtx<Self::Services>,
        mut state: StateEnvelope,
    ) -> Result<StateEnvelope, StepError> {
        let spec: NormalizedSpec = get_typed(&self.input_key, &state).map_err(|e| {
            StepError::transformer("llm_scope", format!("Failed to get normalized_spec: {}", e))
        })?;

        let prompt = self.render_prompt(&spec);
        let report: ScopeReport =
            call_and_decode(&self.handle, &self.services, self.name(), prompt)?;
        put_typed(self.output_key.clone(), report, &mut state);
        Ok(state)
    }
}

pub struct LlmSkeletonStep<S: Services> {
    spec_key: ArtifactKey,
    scope_key: ArtifactKey,
    output_key: ArtifactKey,
    services: S,
    handle: Handle,
}

impl<S: Services> LlmSkeletonStep<S> {
    pub fn new(services: S, handle: Handle) -> Self {
        Self {
            spec_key: ArtifactKey::new("normalized_spec"),
            scope_key: ArtifactKey::new("scope_report"),
            output_key: ArtifactKey::new("proposal_skeleton"),
            services,
            handle,
        }
    }

    fn render_prompt(&self, spec: &NormalizedSpec, scope: &ScopeReport) -> String {
        let spec_json = serde_json::to_string_pretty(spec).unwrap_or_default();
        let scope_json = serde_json::to_string_pretty(scope).unwrap_or_default();
        SKELETON_PROMPT
            .replace("{normalized_spec}", &spec_json)
            .replace("{scope_report}", &scope_json)
    }
}

impl<S: Services + Send + Sync> Transformer for LlmSkeletonStep<S> {
    type Services = S;

    fn name(&self) -> &'static str {
        "llm_skeleton"
    }

    fn transform(
        &self,
        _ctx: &mut naaf_core::budget::ExecCtx<Self::Services>,
        mut state: StateEnvelope,
    ) -> Result<StateEnvelope, StepError> {
        let spec: NormalizedSpec = get_typed(&self.spec_key, &state).map_err(|e| {
            StepError::transformer("llm_skeleton", format!("Failed to get spec: {}", e))
        })?;
        let scope: ScopeReport = get_typed(&self.scope_key, &state).map_err(|e| {
            StepError::transformer("llm_skeleton", format!("Failed to get scope: {}", e))
        })?;

        let prompt = self.render_prompt(&spec, &scope);
        let skeleton: ProposalSkeleton =
            call_and_decode(&self.handle, &self.services, self.name(), prompt)?;
        put_typed(self.output_key.clone(), skeleton, &mut state);
        Ok(state)
    }
}

pub struct LlmAcceptanceStep<S: Services> {
    spec_key: ArtifactKey,
    skeleton_key: ArtifactKey,
    output_key: ArtifactKey,
    services: S,
    handle: Handle,
}

impl<S: Services> LlmAcceptanceStep<S> {
    pub fn new(services: S, handle: Handle) -> Self {
        Self {
            spec_key: ArtifactKey::new("normalized_spec"),
            skeleton_key: ArtifactKey::new("proposal_skeleton"),
            output_key: ArtifactKey::new("acceptance_criteria"),
            services,
            handle,
        }
    }

    fn render_prompt(&self, spec: &NormalizedSpec, skeleton: &ProposalSkeleton) -> String {
        let spec_json = serde_json::to_string_pretty(spec).unwrap_or_default();
        let skeleton_json = serde_json::to_string_pretty(skeleton).unwrap_or_default();
        ACCEPTANCE_PROMPT
            .replace("{normalized_spec}", &spec_json)
            .replace("{proposal_skeleton}", &skeleton_json)
    }
}

impl<S: Services + Send + Sync> Transformer for LlmAcceptanceStep<S> {
    type Services = S;

    fn name(&self) -> &'static str {
        "llm_acceptance"
    }

    fn transform(
        &self,
        _ctx: &mut naaf_core::budget::ExecCtx<Self::Services>,
        mut state: StateEnvelope,
    ) -> Result<StateEnvelope, StepError> {
        let spec: NormalizedSpec = get_typed(&self.spec_key, &state).map_err(|e| {
            StepError::transformer("llm_acceptance", format!("Failed to get spec: {}", e))
        })?;
        let skeleton: ProposalSkeleton = get_typed(&self.skeleton_key, &state).map_err(|e| {
            StepError::transformer("llm_acceptance", format!("Failed to get skeleton: {}", e))
        })?;

        let prompt = self.render_prompt(&spec, &skeleton);
        let criteria: AcceptanceCriteriaSet =
            call_and_decode(&self.handle, &self.services, self.name(), prompt)?;
        put_typed(self.output_key.clone(), criteria, &mut state);
        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm_json::extract_json;
    use crate::test_services::NoopServices;
    use naaf_core::budget::ExecCtx;
    use naaf_schema::execution_status::ExecutionStatus;
    use naaf_schema::lineage::Lineage;
    use naaf_schema::state::{RunId, StateEnvelope, StateId};
    use naaf_schema::state_kind::StateKind;

    #[test]
    fn test_extract_json() {
        let text = "Some text before {\"key\": \"value\"} some text after";
        let json = extract_json(text).unwrap();
        assert_eq!(json, "{\"key\": \"value\"}");
    }

    #[test]
    fn test_extract_json_nested() {
        let text = r#"Before {"outer": {"inner": "value"}} After"#;
        let json = extract_json(text).unwrap();
        assert_eq!(json, r#"{"outer": {"inner": "value"}}"#);
    }

    #[test]
    fn test_extract_json_array() {
        let text = r#"Response: [{"id": 1}, {"id": 2}]"#;
        let json = extract_json(text).unwrap();
        assert_eq!(json, r#"[{"id": 1}, {"id": 2}]"#);
    }

    #[test]
    fn test_extract_json_no_json() {
        assert!(extract_json("No JSON here").is_err());
    }

    #[test]
    fn test_extract_json_malformed() {
        assert!(extract_json(r#"{"incomplete": "#).is_err());
    }

    fn make_empty_state() -> StateEnvelope {
        StateEnvelope::new(
            StateId::new(),
            RunId::new(),
            StateKind::Proposed,
            Lineage::new(None, None, ExecutionStatus::Pending),
        )
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_llm_normalize_step_structure() {
        let step = LlmNormalizeStep::new(NoopServices, Handle::current());
        assert_eq!(step.name(), "llm_normalize");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_llm_scope_step_structure() {
        let step = LlmScopeStep::new(NoopServices, Handle::current());
        assert_eq!(step.name(), "llm_scope");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_llm_skeleton_step_structure() {
        let step = LlmSkeletonStep::new(NoopServices, Handle::current());
        assert_eq!(step.name(), "llm_skeleton");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_llm_acceptance_step_structure() {
        let step = LlmAcceptanceStep::new(NoopServices, Handle::current());
        assert_eq!(step.name(), "llm_acceptance");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_llm_normalize_missing_input() {
        let step = LlmNormalizeStep::new(NoopServices, Handle::current());
        let mut ctx = ExecCtx::new(RunId::new(), NoopServices);
        assert!(step.transform(&mut ctx, make_empty_state()).is_err());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_llm_scope_missing_input() {
        let step = LlmScopeStep::new(NoopServices, Handle::current());
        let mut ctx = ExecCtx::new(RunId::new(), NoopServices);
        assert!(step.transform(&mut ctx, make_empty_state()).is_err());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_llm_skeleton_missing_input() {
        let step = LlmSkeletonStep::new(NoopServices, Handle::current());
        let mut ctx = ExecCtx::new(RunId::new(), NoopServices);
        assert!(step.transform(&mut ctx, make_empty_state()).is_err());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_llm_acceptance_missing_input() {
        let step = LlmAcceptanceStep::new(NoopServices, Handle::current());
        let mut ctx = ExecCtx::new(RunId::new(), NoopServices);
        assert!(step.transform(&mut ctx, make_empty_state()).is_err());
    }
}
