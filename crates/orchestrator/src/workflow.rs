//! Workflow graph definition and execution.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use model::{GenerationRequest, Message, ModelProvider};

use crate::artifact::{Artifact, ArtifactId, ArtifactKind};
use crate::run::Phase;
use openspec::decode;
use openspec::workers::WorkerSpec;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PhaseNode {
    pub phase: Phase,
    pub requires_artifact: Option<ArtifactKind>,
    pub produces_artifact: Option<ArtifactKind>,
}

impl PhaseNode {
    pub fn milestone(phase: Phase) -> Self {
        Self {
            phase,
            requires_artifact: None,
            produces_artifact: None,
        }
    }

    pub fn consumes(phase: Phase, artifact: ArtifactKind) -> Self {
        Self {
            phase,
            requires_artifact: Some(artifact),
            produces_artifact: None,
        }
    }

    pub fn produces(phase: Phase, artifact: ArtifactKind) -> Self {
        Self {
            phase,
            requires_artifact: None,
            produces_artifact: Some(artifact),
        }
    }
}

pub trait ExecutionEngine: Send + Sync {
    fn execute_transition(
        &self,
        run: &mut crate::run::Run,
        spec: &openspec::TransitionSpec,
    ) -> Result<crate::artifact::Artifact, EngineError>;

    fn can_execute(&self, run: &crate::run::Run, spec: &openspec::TransitionSpec) -> bool;
}

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("Transition {0} not allowed from phase {1}")]
    DisallowedTransition(String, Phase),

    #[error("Missing required artifact: {0:?}")]
    MissingArtifact(ArtifactKind),

    #[error("Retry limit exceeded for {0}")]
    RetryLimitExceeded(String),

    #[error("Timeout after {0} seconds")]
    Timeout(u64),

    #[error("Worker execution failed: {0}")]
    WorkerFailed(String),

    #[error("Model provider error: {0}")]
    ModelError(#[from] model::ProviderError),

    #[error("Failed to parse worker output: {0}")]
    ParseError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Store error: {0}")]
    Store(#[from] crate::store::StoreError),

    #[error("Journal error: {0}")]
    Journal(#[from] crate::journal::JournalError),
}

pub struct ModelClient {
    provider: Arc<dyn ModelProvider>,
}

impl ModelClient {
    pub fn new(provider: Arc<dyn ModelProvider>) -> Self {
        Self { provider }
    }

    pub fn provider(&self) -> Arc<dyn ModelProvider> {
        Arc::clone(&self.provider)
    }
}

pub struct WorkerExecutor {
    provider: Arc<dyn ModelProvider>,
    model: String,
}

impl WorkerExecutor {
    pub fn new(provider: Arc<dyn ModelProvider>, model: String) -> Self {
        Self { provider, model }
    }

    pub fn render_prompt(&self, spec: &WorkerSpec, artifacts: &[(&Artifact, &[u8])]) -> String {
        let mut prompt = spec.prompt_template.to_string();

        for (artifact, content) in artifacts {
            let content_str = String::from_utf8_lossy(content);
            let placeholder = format!("{{{}}}", artifact.kind.name());
            prompt = prompt.replace(&placeholder, &content_str);
        }

        prompt
    }

    pub fn execute(
        &self,
        spec: &WorkerSpec,
        artifacts: &[(&Artifact, &[u8])],
    ) -> Result<String, EngineError> {
        let prompt = self.render_prompt(spec, artifacts);

        let request = GenerationRequest::new(self.model.clone(), vec![Message::user(prompt)]);

        let response = self
            .provider
            .generate(request)
            .map_err(EngineError::ModelError)?;

        Ok(response.content)
    }
}

pub struct DefaultExecutionEngine {
    executor: WorkerExecutor,
    store: crate::store::ArtifactStore,
    journal: crate::journal::Journal,
}

impl DefaultExecutionEngine {
    pub fn new(
        provider: Arc<dyn ModelProvider>,
        model: String,
        store: crate::store::ArtifactStore,
        journal: crate::journal::Journal,
    ) -> Self {
        Self {
            executor: WorkerExecutor::new(provider, model),
            store,
            journal,
        }
    }

    pub fn executor(&self) -> &WorkerExecutor {
        &self.executor
    }

    pub fn store(&self) -> &crate::store::ArtifactStore {
        &self.store
    }

    pub fn journal(&self) -> &crate::journal::Journal {
        &self.journal
    }
}

impl ExecutionEngine for DefaultExecutionEngine {
    fn execute_transition(
        &self,
        run: &mut crate::run::Run,
        spec: &openspec::TransitionSpec,
    ) -> Result<crate::artifact::Artifact, EngineError> {
        self.execute_transition_with_retry(run, spec, spec.retry_limit)
    }

    fn can_execute(&self, run: &crate::run::Run, spec: &openspec::TransitionSpec) -> bool {
        run.phase == spec.from_phase
    }
}

impl DefaultExecutionEngine {
    fn execute_transition_with_retry(
        &self,
        run: &mut crate::run::Run,
        spec: &openspec::TransitionSpec,
        remaining_retries: u32,
    ) -> Result<crate::artifact::Artifact, EngineError> {
        match self.execute_transition_once(run, spec) {
            Ok(artifact) => Ok(artifact),
            Err(e) => {
                if remaining_retries > 0 {
                    tracing::warn!(
                        "Transition {} failed, {} retries remaining: {}",
                        spec.name,
                        remaining_retries,
                        e
                    );
                    self.execute_transition_with_retry(run, spec, remaining_retries - 1)
                } else {
                    Err(e)
                }
            }
        }
    }

    #[tracing::instrument(skip(self, run, spec), fields(run_id = %run.id, from_phase = ?run.phase, to_phase = ?spec.to_phase, transition_name = %spec.name))]
    fn execute_transition_once(
        &self,
        run: &mut crate::run::Run,
        spec: &openspec::TransitionSpec,
    ) -> Result<crate::artifact::Artifact, EngineError> {
        let artifacts: Vec<(Artifact, Vec<u8>)> = self.load_required_artifacts(run, spec)?;

        let worker_spec = self.find_worker_spec(&spec.worker_id)?;

        let artifact_refs: Vec<(&Artifact, &[u8])> = artifacts
            .iter()
            .map(|(artifact, content)| (artifact, content.as_slice()))
            .collect();

        let output = self.executor.execute(&worker_spec, &artifact_refs)?;

        let content = self.decode_output(&spec.produces, &output)?;

        let parent_ids: Vec<ArtifactId> = artifacts.iter().map(|(a, _)| a.id).collect();

        let artifact = Artifact::new(run.id, spec.produces, parent_ids, run.worktree.clone());

        self.store.save(&artifact, content.as_bytes())?;

        run.transition_to(spec.to_phase);

        let event = crate::journal::transition_executed(
            run.id,
            spec.from_phase,
            spec.to_phase,
            &spec.worker_id,
            Some(artifact.id),
        );
        self.journal.append(&event)?;

        Ok(artifact)
    }

    fn load_required_artifacts(
        &self,
        run: &crate::run::Run,
        spec: &openspec::TransitionSpec,
    ) -> Result<Vec<(Artifact, Vec<u8>)>, EngineError> {
        let refs = self.store.list(run.id)?;

        let mut artifacts = Vec::new();
        for &kind in &spec.consumes {
            let matching: Vec<_> = refs.iter().filter(|r| r.kind == kind).collect();

            if matching.is_empty() {
                return Err(EngineError::MissingArtifact(kind));
            }

            let latest = matching.last().unwrap();
            let (artifact, content) = self.store.load(latest.id, run.id)?;
            artifacts.push((artifact, content));
        }

        Ok(artifacts)
    }

    fn find_worker_spec(&self, worker_id: &str) -> Result<WorkerSpec, EngineError> {
        for spec in openspec::all_worker_specs() {
            if spec.id.name() == worker_id {
                return Ok(spec);
            }
        }
        Err(EngineError::WorkerFailed(format!(
            "Unknown worker: {}",
            worker_id
        )))
    }

    fn decode_output(&self, produces: &ArtifactKind, output: &str) -> Result<String, EngineError> {
        match produces {
            ArtifactKind::NormalizedSpec => decode::decode_normalized_spec(output)
                .map(|s| serde_json::to_string_pretty(&s).unwrap())
                .map_err(|e| EngineError::ParseError(e.to_string())),
            ArtifactKind::ScopeReport => decode::decode_scope_report(output)
                .map(|s| serde_json::to_string_pretty(&s).unwrap())
                .map_err(|e| EngineError::ParseError(e.to_string())),
            ArtifactKind::ProposalSkeleton => decode::decode_proposal_skeleton(output)
                .map(|s| serde_json::to_string_pretty(&s).unwrap())
                .map_err(|e| EngineError::ParseError(e.to_string())),
            ArtifactKind::AcceptanceCriteriaSet => decode::decode_acceptance_criteria(output)
                .map(|s| serde_json::to_string_pretty(&s).unwrap())
                .map_err(|e| EngineError::ParseError(e.to_string())),
            _ => Ok(output.to_string()),
        }
    }
}

#[tracing::instrument(skip(engine, workflow, run), fields(run_id = %run.id, initial_phase = ?run.phase))]
pub fn run_workflow(
    engine: &DefaultExecutionEngine,
    workflow: &openspec::WorkflowDefinition,
    run: &mut crate::run::Run,
) -> Result<crate::run::Outcome, EngineError> {
    let mut current_phase = run.phase;

    while !workflow.is_terminal_phase(current_phase) {
        let transitions = workflow.outgoing_transitions(current_phase);

        if transitions.is_empty() {
            break;
        }

        let spec = &transitions[0];

        if !engine.can_execute(run, spec) {
            tracing::warn!(
                from_phase = ?current_phase,
                transition_name = %spec.name,
                "Cannot execute transition - skipping"
            );
            return Ok(run.outcome.clone());
        }

        match engine.execute_transition(run, spec) {
            Ok(_) => {
                current_phase = run.phase;
            }
            Err(e) => {
                tracing::error!(
                    from_phase = ?spec.from_phase,
                    to_phase = ?spec.to_phase,
                    transition_name = %spec.name,
                    error = %e,
                    "Transition failed"
                );
                run.fail(crate::run::TerminalReason::Failed {
                    message: format!("Transition {} failed: {}", spec.name, e),
                });
                return Ok(run.outcome.clone());
            }
        }
    }

    if workflow.is_terminal_phase(run.phase) {
        run.complete();
        tracing::info!(
            outcome = "done",
            final_phase = ?run.phase,
            "Workflow completed successfully"
        );
    }

    Ok(run.outcome.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use openspec::workers::{WorkerId, WorkerSpec};

    #[test]
    fn test_render_prompt_with_single_artifact() {
        let provider = Arc::new(MockProvider::new());
        let executor = WorkerExecutor::new(provider, "test-model".to_string());

        let spec = WorkerSpec {
            id: WorkerId::RequestNormalizer,
            consumes: vec![openspec::ArtifactKind::UserPrompt],
            produces: openspec::ArtifactKind::NormalizedSpec,
            prompt_template: "Input: {user_prompt}\nOutput:",
            success_criteria: vec![],
        };

        let artifact = Artifact::new(
            crate::run::RunId::new(),
            openspec::ArtifactKind::UserPrompt,
            vec![],
            std::path::PathBuf::from("test.bin"),
        );
        let content = b"Build a login system";

        let prompt = executor.render_prompt(&spec, &[(&artifact, content)]);

        assert!(prompt.contains("Build a login system"));
    }

    #[test]
    fn test_render_prompt_with_multiple_artifacts() {
        let provider = Arc::new(MockProvider::new());
        let executor = WorkerExecutor::new(provider, "test-model".to_string());

        let spec = WorkerSpec {
            id: WorkerId::ProposalSkeletonBuilder,
            consumes: vec![
                openspec::ArtifactKind::NormalizedSpec,
                openspec::ArtifactKind::ScopeReport,
            ],
            produces: openspec::ArtifactKind::ProposalSkeleton,
            prompt_template: "Spec: {normalized_spec}\nScope: {scope_report}\nBuild proposal:",
            success_criteria: vec![],
        };

        let artifact1 = Artifact::new(
            crate::run::RunId::new(),
            openspec::ArtifactKind::NormalizedSpec,
            vec![],
            std::path::PathBuf::from("spec.bin"),
        );
        let content1 = b"Add authentication";

        let artifact2 = Artifact::new(
            crate::run::RunId::new(),
            openspec::ArtifactKind::ScopeReport,
            vec![],
            std::path::PathBuf::from("scope.bin"),
        );
        let content2 = b"In-scope: login, logout";

        let prompt =
            executor.render_prompt(&spec, &[(&artifact1, content1), (&artifact2, content2)]);

        assert!(prompt.contains("Add authentication"));
        assert!(prompt.contains("In-scope: login, logout"));
    }

    struct MockProvider;

    impl MockProvider {
        fn new() -> Self {
            Self
        }
    }

    impl ModelProvider for MockProvider {
        fn generate(
            &self,
            _request: model::types::GenerationRequest,
        ) -> std::result::Result<model::types::GenerationResponse, model::ProviderError> {
            Ok(model::types::GenerationResponse {
                content: "mock response".to_string(),
                model: "test".to_string(),
                usage: model::types::Usage {
                    prompt_tokens: 10,
                    completion_tokens: 20,
                    total_tokens: 30,
                },
                finish_reason: "stop".to_string(),
            })
        }

        fn capabilities(&self) -> model::types::ProviderCapabilities {
            model::types::ProviderCapabilities::new(false, 1000)
        }
    }
}
