//! Artifact types and storage interfaces.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ArtifactId(pub Uuid);

impl ArtifactId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ArtifactId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ArtifactId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArtifactKind {
    UserPrompt,
    NormalizedSpec,
    ScopeReport,
    ProposalSkeleton,
    AcceptanceCriteriaSet,
    TaskPlan,
    CandidatePatch,
    ValidationResults,
    ReviewFindings,
    RemediationPlan,
    DeliveryBundle,
}

impl ArtifactKind {
    pub fn name(&self) -> &'static str {
        match self {
            ArtifactKind::UserPrompt => "user_prompt",
            ArtifactKind::NormalizedSpec => "normalized_spec",
            ArtifactKind::ScopeReport => "scope_report",
            ArtifactKind::ProposalSkeleton => "proposal_skeleton",
            ArtifactKind::AcceptanceCriteriaSet => "acceptance_criteria_set",
            ArtifactKind::TaskPlan => "task_plan",
            ArtifactKind::CandidatePatch => "candidate_patch",
            ArtifactKind::ValidationResults => "validation_results",
            ArtifactKind::ReviewFindings => "review_findings",
            ArtifactKind::RemediationPlan => "remediation_plan",
            ArtifactKind::DeliveryBundle => "delivery_bundle",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactRef {
    pub id: ArtifactId,
    pub kind: ArtifactKind,
}

impl ArtifactRef {
    pub fn new(id: ArtifactId, kind: ArtifactKind) -> Self {
        Self { id, kind }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: ArtifactId,
    pub run_id: super::run::RunId,
    pub kind: ArtifactKind,
    pub parent_ids: Vec<ArtifactId>,
    pub content_path: PathBuf,
    pub created_at: DateTime<Utc>,
}

impl Artifact {
    pub fn new(
        run_id: super::run::RunId,
        kind: ArtifactKind,
        parent_ids: Vec<ArtifactId>,
        content_path: PathBuf,
    ) -> Self {
        Self {
            id: ArtifactId::new(),
            run_id,
            kind,
            parent_ids,
            content_path,
            created_at: Utc::now(),
        }
    }

    pub fn ref_of(&self) -> ArtifactRef {
        ArtifactRef::new(self.id, self.kind)
    }
}
