//! Remediation loop implementation.
//!
//! # Legacy Code
//!
//! This module is part of the legacy prototype runtime.
//! **Do not build new features on this code.**
//! See the repository root `LEGACY.md` for details.

use std::collections::HashSet;
use std::sync::Arc;

use chrono::Utc;
use naaf_openspec::workers::{
    consistency_reviewer_spec, findings_aggregator_spec, readiness_evaluator_spec,
    remediation_planner_spec, risk_reviewer_spec, targeted_remediator_spec,
};
use serde::{Deserialize, Serialize};

use naaf_model::{ModelProvider, ProviderError};

use crate::artifact::{Artifact, ArtifactId, ArtifactKind};
use crate::finding::{Finding, FindingId, FindingStatus, Severity};
use crate::journal::Event;
use crate::run::{Outcome, Phase, Run, RunId, TerminalReason};
use crate::store::{ArtifactStore, FindingStore};
use crate::workflow::WorkerExecutor;

const MAX_REMEDIATION_ITERATIONS: u32 = 2;
const ESCALATION_FINDING_THRESHOLD: usize = 10;

const DECISION_ACCEPTED: &str = "accepted";
const DECISION_ESCALATED: &str = "escalated";
const DECISION_REJECTED: &str = "rejected";

#[derive(Debug, thiserror::Error)]
pub enum RemediationError {
    #[error("Worker execution failed: {0}")]
    WorkerFailed(String),

    #[error("Model provider error: {0}")]
    ModelError(#[from] ProviderError),

    #[error("Engine error: {0}")]
    Engine(#[from] crate::workflow::EngineError),

    #[error("Store error: {0}")]
    Store(#[from] crate::store::StoreError),

    #[error("Journal error: {0}")]
    Journal(#[from] crate::journal::JournalError),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("UTF-8 conversion error: {0}")]
    Utf8Error(String),

    #[error("No proposal found")]
    NoProposal,

    #[error("No findings artifact")]
    NoFindingsArtifact,

    #[error("Parallel execution error: {0}")]
    Parallel(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type RemediationResult<T> = Result<T, RemediationError>;

#[derive(Debug, Clone)]
pub struct FindingSet {
    pub findings: Vec<Finding>,
}

#[derive(Debug, Deserialize)]
struct AggregatedFindings {
    findings: Vec<AggregatedFinding>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AggregatedFinding {
    id: String,
    category: String,
    severity: String,
    evidence: Vec<String>,
    impacted_sections: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct RemediationPlanOutput {
    selected_finding_id: String,
    cluster_ids: Vec<String>,
    should_escalate: bool,
    reason: String,
}

#[derive(Debug, Deserialize)]
struct RemediationPatch {
    target_sections: Vec<String>,
    replacement_text: String,
}

#[derive(Debug, Deserialize)]
struct ReadinessDecision {
    decision: String,
    reasons: Vec<String>,
}

pub struct RemediationEngine<P: ModelProvider> {
    executor: Arc<WorkerExecutor<P>>,
    artifact_store: ArtifactStore,
    finding_store: FindingStore,
    journal: crate::journal::Journal,
}

impl<P: ModelProvider + Sync> RemediationEngine<P> {
    pub fn new(
        provider: Arc<P>,
        model: String,
        artifact_store: ArtifactStore,
        finding_store: FindingStore,
        journal: crate::journal::Journal,
    ) -> Self {
        Self {
            executor: Arc::new(WorkerExecutor::new(provider, model)),
            artifact_store,
            finding_store,
            journal,
        }
    }

    pub async fn execute_review_transitions(&self, run: &mut Run) -> RemediationResult<FindingSet> {
        self.record_review_started()?;

        let proposal = self.load_latest_proposal(run.id)?;
        let content = self.load_artifact_content(&proposal)?;

        let risk_spec = risk_reviewer_spec();
        let consistency_spec = consistency_reviewer_spec();

        let executor = Arc::clone(&self.executor);
        let proposal_clone = proposal.clone();
        let content_clone = content.clone();
        let risk_spec_clone = risk_spec.clone();

        let executor2 = Arc::clone(&self.executor);
        let proposal_clone2 = proposal.clone();
        let content_clone2 = content.clone();
        let consistency_spec_clone = consistency_spec.clone();

        let risk_future = async move {
            let content_bytes = content_clone.as_bytes();
            executor
                .execute(&risk_spec_clone, &[(&proposal_clone, content_bytes)])
                .await
        };

        let consistency_future = async move {
            let content_bytes = content_clone2.as_bytes();
            executor2
                .execute(
                    &consistency_spec_clone,
                    &[(&proposal_clone2, content_bytes)],
                )
                .await
        };

        let (risk_result, consistency_result) = tokio::join!(risk_future, consistency_future);

        let risk_findings = risk_result?;
        let consistency_findings = consistency_result?;

        let aggregated = self
            .aggregate_findings(run, &risk_findings, &consistency_findings)
            .await?;

        let findings = self.persist_findings(run, &aggregated)?;

        self.record_finding_created_events(&findings)?;

        Ok(FindingSet { findings })
    }

    fn record_review_started(&self) -> RemediationResult<()> {
        let event = Event::ReviewStarted {
            timestamp: Utc::now(),
        };
        self.journal.append(&event)?;
        Ok(())
    }

    fn load_latest_proposal(&self, run_id: RunId) -> RemediationResult<Artifact> {
        let refs = self.artifact_store.list(run_id)?;

        let proposal_refs: Vec<_> = refs
            .iter()
            .filter(|r| {
                r.kind == ArtifactKind::ProposalSkeleton || r.kind == ArtifactKind::CurrentProposal
            })
            .collect();

        let latest = proposal_refs.last().ok_or(RemediationError::NoProposal)?;

        let (artifact, _) = self.artifact_store.load(latest.id, run_id)?;
        Ok(artifact)
    }

    fn load_artifact_content(&self, artifact: &Artifact) -> RemediationResult<String> {
        let (_artifact, content) = self.artifact_store.load(artifact.id, artifact.run_id)?;
        String::from_utf8(content).map_err(|e| RemediationError::Utf8Error(e.to_string()))
    }

    async fn aggregate_findings(
        &self,
        run: &mut Run,
        risk_findings: &str,
        consistency_findings: &str,
    ) -> RemediationResult<AggregatedFindings> {
        let spec = findings_aggregator_spec();

        let risk_artifact = Artifact::new(
            run.id,
            ArtifactKind::RiskFindings,
            vec![],
            std::path::PathBuf::from("risk_findings.json"),
        );
        let consistency_artifact = Artifact::new(
            run.id,
            ArtifactKind::ConsistencyFindings,
            vec![],
            std::path::PathBuf::from("consistency_findings.json"),
        );

        let output = self
            .executor
            .execute(
                &spec,
                &[
                    (&risk_artifact, risk_findings.as_bytes()),
                    (&consistency_artifact, consistency_findings.as_bytes()),
                ],
            )
            .await?;

        let parsed: AggregatedFindings = serde_json::from_str(&output)
            .map_err(|e| RemediationError::ParseError(e.to_string()))?;

        Ok(parsed)
    }

    fn persist_findings(
        &self,
        run: &mut Run,
        aggregated: &AggregatedFindings,
    ) -> RemediationResult<Vec<Finding>> {
        let mut findings = Vec::new();

        for af in &aggregated.findings {
            let severity = match af.severity.to_lowercase().as_str() {
                "high" => Severity::High,
                "medium" => Severity::Medium,
                _ => Severity::Low,
            };

            let finding = Finding::new(
                run.id,
                af.id.clone(),
                severity,
                af.category.clone(),
                af.evidence.clone(),
                vec![],
            );

            self.finding_store.save(&finding)?;
            findings.push(finding);
        }

        Ok(findings)
    }

    fn record_finding_created_events(&self, findings: &[Finding]) -> RemediationResult<()> {
        for finding in findings {
            let event = Event::FindingCreated {
                finding_id: finding.id,
                severity: finding.severity,
                category: finding.category.clone(),
                timestamp: Utc::now(),
            };
            self.journal.append(&event)?;
        }
        Ok(())
    }

    pub async fn execute_remediation_cycle(
        &self,
        run: &mut Run,
        findings: &FindingSet,
    ) -> RemediationResult<RemediationCycleOutcome> {
        if findings.findings.is_empty() {
            return Ok(RemediationCycleOutcome::Accept);
        }

        let proposal = self.load_latest_proposal(run.id)?;
        let findings_artifact = self.create_findings_artifact(run, findings, &[proposal.id])?;

        let plan_output = self.execute_remediation_planner(&findings_artifact).await?;

        if plan_output.should_escalate {
            return Ok(RemediationCycleOutcome::Escalate {
                reason: plan_output.reason,
            });
        }

        let patch_artifact = self
            .execute_targeted_remediator(run, &proposal, &plan_output)
            .await?;

        self.apply_patch(run, &proposal, &patch_artifact)?;

        self.update_finding_statuses(run, findings)?;

        self.record_finding_resolved_events(findings)?;

        let decision = self
            .execute_readiness_evaluator(run, &proposal, &patch_artifact, findings)
            .await?;

        match decision.decision.to_lowercase().as_str() {
            DECISION_ACCEPTED => Ok(RemediationCycleOutcome::Accept),
            DECISION_ESCALATED => Ok(RemediationCycleOutcome::Escalate {
                reason: decision.reasons.join("; "),
            }),
            DECISION_REJECTED => Ok(RemediationCycleOutcome::Reject {
                reason: decision.reasons.join("; "),
            }),
            _ => Ok(RemediationCycleOutcome::Continue),
        }
    }

    fn create_findings_artifact(
        &self,
        run: &mut Run,
        findings: &FindingSet,
        parent_ids: &[ArtifactId],
    ) -> RemediationResult<Artifact> {
        let findings_json = serde_json::to_string(&findings.findings)
            .map_err(|e| RemediationError::ParseError(e.to_string()))?;

        let artifact = Artifact::new(
            run.id,
            ArtifactKind::ReviewFindings,
            parent_ids.to_vec(),
            run.worktree.clone(),
        );

        self.artifact_store
            .save(&artifact, findings_json.as_bytes())?;

        Ok(artifact)
    }

    async fn execute_remediation_planner(
        &self,
        findings_artifact: &Artifact,
    ) -> RemediationResult<RemediationPlanOutput> {
        let spec = remediation_planner_spec();
        let content = self.load_artifact_content(findings_artifact)?;
        let output = self
            .executor
            .execute(&spec, &[(findings_artifact, content.as_bytes())])
            .await?;

        let parsed: RemediationPlanOutput = serde_json::from_str(&output)
            .map_err(|e| RemediationError::ParseError(e.to_string()))?;

        Ok(parsed)
    }

    async fn execute_targeted_remediator(
        &self,
        run: &mut Run,
        proposal: &Artifact,
        plan: &RemediationPlanOutput,
    ) -> RemediationResult<Artifact> {
        let spec = targeted_remediator_spec();

        let proposal_content = self.load_artifact_content(proposal)?;

        let plan_artifact = Artifact::new(
            run.id,
            ArtifactKind::RemediationPlan,
            vec![proposal.id],
            run.worktree.clone(),
        );
        let plan_json =
            serde_json::to_string(plan).map_err(|e| RemediationError::ParseError(e.to_string()))?;
        self.artifact_store
            .save(&plan_artifact, plan_json.as_bytes())?;

        let output = self
            .executor
            .execute(
                &spec,
                &[
                    (proposal, proposal_content.as_bytes()),
                    (&plan_artifact, plan_json.as_bytes()),
                ],
            )
            .await?;

        let patch_artifact = Artifact::new(
            run.id,
            ArtifactKind::CandidatePatch,
            vec![proposal.id],
            run.worktree.clone(),
        );

        self.artifact_store
            .save(&patch_artifact, output.as_bytes())?;

        Ok(patch_artifact)
    }

    fn apply_patch(
        &self,
        run: &mut Run,
        proposal: &Artifact,
        patch_artifact: &Artifact,
    ) -> RemediationResult<()> {
        let (_proposal_artifact, proposal_content) =
            self.artifact_store.load(proposal.id, run.id)?;
        let (_patch_artifact, patch_content) =
            self.artifact_store.load(patch_artifact.id, run.id)?;

        let patch: RemediationPatch = serde_json::from_slice(&patch_content)
            .map_err(|e| RemediationError::ParseError(e.to_string()))?;

        let mut new_content = String::from_utf8(proposal_content)
            .map_err(|e| RemediationError::Utf8Error(e.to_string()))?;

        for target_section in &patch.target_sections {
            let section_marker = format!("## {}", target_section);
            if let Some(start_pos) = new_content.find(&section_marker) {
                let content_start = start_pos + section_marker.len();

                let next_section_pos = new_content[content_start..]
                    .find("\n## ")
                    .map(|p| content_start + p)
                    .unwrap_or(new_content.len());

                new_content.replace_range(start_pos..next_section_pos, &patch.replacement_text);
            }
        }

        let new_proposal = Artifact::new(
            run.id,
            ArtifactKind::CurrentProposal,
            vec![proposal.id],
            run.worktree.clone(),
        );

        self.artifact_store
            .save(&new_proposal, new_content.as_bytes())?;

        run.transition_to(Phase::ReadyForRemediation);

        Ok(())
    }

    fn update_finding_statuses(&self, run: &Run, findings: &FindingSet) -> RemediationResult<()> {
        for finding in &findings.findings {
            self.finding_store
                .update_status(finding.id, run.id, FindingStatus::Resolved)?;
        }
        Ok(())
    }

    fn record_finding_resolved_events(&self, findings: &FindingSet) -> RemediationResult<()> {
        for finding in &findings.findings {
            let event = Event::FindingResolved {
                finding_id: finding.id,
                timestamp: Utc::now(),
            };
            self.journal.append(&event)?;
        }
        Ok(())
    }

    fn record_run_escalated(&self, reason: &str) -> RemediationResult<()> {
        let event = Event::RunEscalated {
            reason: reason.to_string(),
            timestamp: Utc::now(),
        };
        self.journal.append(&event)?;
        Ok(())
    }

    async fn execute_readiness_evaluator(
        &self,
        run: &Run,
        proposal: &Artifact,
        patch_artifact: &Artifact,
        findings: &FindingSet,
    ) -> RemediationResult<ReadinessDecision> {
        let spec = readiness_evaluator_spec();

        let proposal_content = self.load_artifact_content(proposal)?;
        let patch_content = self.artifact_store.load(patch_artifact.id, run.id)?.1;
        let findings_json = serde_json::to_string(&findings.findings)
            .map_err(|e| RemediationError::ParseError(e.to_string()))?;

        let proposal_artifact = Artifact::new(
            run.id,
            ArtifactKind::CurrentProposal,
            vec![proposal.id],
            run.worktree.clone(),
        );
        let findings_input_artifact = Artifact::new(
            run.id,
            ArtifactKind::ReviewFindings,
            vec![],
            run.worktree.clone(),
        );

        let output = self
            .executor
            .execute(
                &spec,
                &[
                    (&proposal_artifact, proposal_content.as_bytes()),
                    (proposal, proposal_content.as_bytes()),
                    (patch_artifact, &patch_content),
                    (&findings_input_artifact, findings_json.as_bytes()),
                ],
            )
            .await?;

        let parsed: ReadinessDecision = serde_json::from_str(&output)
            .map_err(|e| RemediationError::ParseError(e.to_string()))?;

        Ok(parsed)
    }

    pub async fn run_remediation_loop(&self, run: &mut Run) -> RemediationResult<Outcome> {
        let mut iteration: u32 = 0;
        let mut resolved_finding_ids: HashSet<FindingId> = HashSet::new();

        loop {
            iteration += 1;

            let findings = self.execute_review_transitions(run).await?;

            if findings.findings.is_empty() {
                run.complete();
                return Ok(run.outcome.clone());
            }

            for finding in &findings.findings {
                if resolved_finding_ids.contains(&finding.id) {
                    let reason = format!("Recurring finding: {}", finding.id);
                    run.fail(TerminalReason::Escalated {
                        message: reason.clone(),
                    });
                    self.record_run_escalated(&reason)?;
                    return Ok(run.outcome.clone());
                }
            }

            if findings.findings.len() > ESCALATION_FINDING_THRESHOLD {
                let reason = format!(
                    "Finding count exceeds threshold: {}",
                    findings.findings.len()
                );
                run.fail(TerminalReason::Escalated {
                    message: reason.clone(),
                });
                self.record_run_escalated(&reason)?;
                return Ok(run.outcome.clone());
            }

            resolved_finding_ids.extend(findings.findings.iter().map(|f| f.id));

            let outcome = self.execute_remediation_cycle(run, &findings).await?;

            match outcome {
                RemediationCycleOutcome::Accept => {
                    run.complete();
                    return Ok(run.outcome.clone());
                }
                RemediationCycleOutcome::Escalate { reason } => {
                    run.fail(TerminalReason::Escalated {
                        message: reason.clone(),
                    });
                    self.record_run_escalated(&reason)?;
                    return Ok(run.outcome.clone());
                }
                RemediationCycleOutcome::Reject { reason } => {
                    run.fail(TerminalReason::Failed { message: reason });
                    return Ok(run.outcome.clone());
                }
                RemediationCycleOutcome::Continue => {
                    if iteration >= MAX_REMEDIATION_ITERATIONS {
                        let reason = "Iteration limit reached".to_string();
                        run.fail(TerminalReason::Escalated {
                            message: reason.clone(),
                        });
                        self.record_run_escalated(&reason)?;
                        return Ok(run.outcome.clone());
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum RemediationCycleOutcome {
    Accept,
    Escalate { reason: String },
    Reject { reason: String },
    Continue,
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Arc;

    use naaf_model::{GenerationRequest, GenerationResponse, ProviderCapabilities, Usage};
    use tempfile::TempDir;

    struct MockProvider;

    impl MockProvider {
        fn new() -> Self {
            Self
        }
    }

    impl ModelProvider for MockProvider {
        async fn generate(
            &self,
            _request: GenerationRequest,
        ) -> std::result::Result<GenerationResponse, ProviderError> {
            Ok(GenerationResponse {
                content: r#"{"decision": "accepted", "reasons": [], "next_steps": []}"#.to_string(),
                model: "test".to_string(),
                usage: Usage {
                    prompt_tokens: 10,
                    completion_tokens: 20,
                    total_tokens: 30,
                },
                finish_reason: "stop".to_string(),
            })
        }

        async fn capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities::new(false, 1000)
        }
    }

    fn setup_engine() -> (RemediationEngine<MockProvider>, TempDir, TempDir) {
        let artifact_dir = TempDir::new().unwrap();
        let finding_dir = TempDir::new().unwrap();
        let journal_dir = TempDir::new().unwrap();

        let artifact_store = ArtifactStore::new(artifact_dir.path()).unwrap();
        let finding_store = FindingStore::new(finding_dir.path()).unwrap();
        let journal = crate::journal::Journal::new(journal_dir.path()).unwrap();
        let provider = Arc::new(MockProvider::new());

        let engine = RemediationEngine::new(
            provider,
            "test-model".to_string(),
            artifact_store,
            finding_store,
            journal,
        );

        (engine, artifact_dir, finding_dir)
    }

    #[test]
    fn test_remediation_engine_creation() {
        let (_engine, _artifact_dir, _finding_dir) = setup_engine();
    }

    #[test]
    fn test_finding_set_creation() {
        let findings = vec![Finding::new(
            crate::run::RunId::new(),
            "RISK-1".to_string(),
            Severity::High,
            "security".to_string(),
            vec!["Test evidence".to_string()],
            vec![],
        )];

        let set = FindingSet { findings };
        assert_eq!(set.findings.len(), 1);
    }

    #[test]
    fn test_remediation_cycle_outcome_variants() {
        let accept = RemediationCycleOutcome::Accept;
        assert!(matches!(accept, RemediationCycleOutcome::Accept));

        let escalate = RemediationCycleOutcome::Escalate {
            reason: "test".to_string(),
        };
        assert!(matches!(escalate, RemediationCycleOutcome::Escalate { .. }));

        let reject = RemediationCycleOutcome::Reject {
            reason: "test".to_string(),
        };
        assert!(matches!(reject, RemediationCycleOutcome::Reject { .. }));

        let cont = RemediationCycleOutcome::Continue;
        assert!(matches!(cont, RemediationCycleOutcome::Continue));
    }

    #[test]
    fn test_max_iterations_constant() {
        assert_eq!(MAX_REMEDIATION_ITERATIONS, 2);
    }

    #[test]
    fn test_escalation_threshold_constant() {
        assert_eq!(ESCALATION_FINDING_THRESHOLD, 10);
    }

    #[test]
    fn test_decision_constants() {
        assert_eq!(DECISION_ACCEPTED, "accepted");
        assert_eq!(DECISION_ESCALATED, "escalated");
        assert_eq!(DECISION_REJECTED, "rejected");
    }

    #[test]
    fn test_no_proposal_error() {
        let result: RemediationResult<()> = Err(RemediationError::NoProposal);
        assert!(matches!(result, Err(RemediationError::NoProposal)));
    }

    #[test]
    fn test_utf8_error_variant() {
        let error = RemediationError::Utf8Error("invalid utf-8".to_string());
        assert!(format!("{}", error).contains("UTF-8"));
    }
}
