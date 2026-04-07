//! LLM-powered workflow steps using OpenSpec prompts.

use crate::prompts;
use crate::{AcceptanceCriteriaSet, NormalizedSpec, ProposalSkeleton, ScopeReport};
use naaf_core::budget::Services;
use naaf_core::errors::StepError;
use naaf_core::steps::Transformer;
use naaf_schema::adapters::{get_typed, put_typed};
use naaf_schema::artifacts::ArtifactKey;
use naaf_schema::state::StateEnvelope;
use serde::de::DeserializeOwned;
use tokio::runtime::Handle;

fn extract_json(text: &str) -> Result<String, StepError> {
    let object_start = text.find('{');
    let array_start = text.find('[');

    let (start_idx, end_char) = match (object_start, array_start) {
        (Some(object), Some(array)) if object < array => (object, '}'),
        (Some(_), Some(array)) => (array, ']'),
        (Some(object), None) => (object, '}'),
        (None, Some(array)) => (array, ']'),
        (None, None) => {
            return Err(StepError::transformer(
                "extract_json",
                "No JSON object or array found in response",
            ));
        }
    };

    let end_idx = text.rfind(end_char).ok_or_else(|| {
        StepError::transformer(
            "extract_json",
            format!("No matching '{}' found for JSON", end_char),
        )
    })?;

    let json = text[start_idx..=end_idx].to_string();
    serde_json::from_str::<serde_json::Value>(&json).map_err(|e| {
        StepError::transformer(
            "extract_json",
            format!("Extracted string is not valid JSON: {}", e),
        )
    })?;

    Ok(json)
}

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
    let json = extract_json(&response)?;

    serde_json::from_str(&json)
        .map_err(|e| StepError::transformer(step_name, format!("JSON parse error: {}", e)))
}

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
        prompts::REQUEST_NORMALIZER_PROMPT.replace("{user_prompt}", input)
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
        prompts::SCOPE_ANALYST_PROMPT.replace("{normalized_spec}", &spec_json)
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
        prompts::SKELETON_BUILDER_PROMPT
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
        prompts::ACCEPTANCE_CRITERIA_PROMPT
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
    use naaf_core::budget::{DummyServices, ExecCtx};
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
        let step = LlmNormalizeStep::new(DummyServices, Handle::current());
        assert_eq!(step.name(), "llm_normalize");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_llm_scope_step_structure() {
        let step = LlmScopeStep::new(DummyServices, Handle::current());
        assert_eq!(step.name(), "llm_scope");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_llm_skeleton_step_structure() {
        let step = LlmSkeletonStep::new(DummyServices, Handle::current());
        assert_eq!(step.name(), "llm_skeleton");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_llm_acceptance_step_structure() {
        let step = LlmAcceptanceStep::new(DummyServices, Handle::current());
        assert_eq!(step.name(), "llm_acceptance");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_llm_normalize_missing_input() {
        let step = LlmNormalizeStep::new(DummyServices, Handle::current());
        let mut ctx = ExecCtx::new(RunId::new(), DummyServices);
        assert!(step.transform(&mut ctx, make_empty_state()).is_err());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_llm_scope_missing_input() {
        let step = LlmScopeStep::new(DummyServices, Handle::current());
        let mut ctx = ExecCtx::new(RunId::new(), DummyServices);
        assert!(step.transform(&mut ctx, make_empty_state()).is_err());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_llm_skeleton_missing_input() {
        let step = LlmSkeletonStep::new(DummyServices, Handle::current());
        let mut ctx = ExecCtx::new(RunId::new(), DummyServices);
        assert!(step.transform(&mut ctx, make_empty_state()).is_err());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_llm_acceptance_missing_input() {
        let step = LlmAcceptanceStep::new(DummyServices, Handle::current());
        let mut ctx = ExecCtx::new(RunId::new(), DummyServices);
        assert!(step.transform(&mut ctx, make_empty_state()).is_err());
    }
}
