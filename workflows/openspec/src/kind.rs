//! Artifact kind definitions for OpenSpec workflow.

use serde::{Deserialize, Serialize};

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
    RiskFindings,
    ConsistencyFindings,
    CurrentProposal,
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
            ArtifactKind::RiskFindings => "risk_findings",
            ArtifactKind::ConsistencyFindings => "consistency_findings",
            ArtifactKind::CurrentProposal => "current_proposal",
            ArtifactKind::RemediationPlan => "remediation_plan",
            ArtifactKind::DeliveryBundle => "delivery_bundle",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_artifact_kind_names() {
        assert_eq!(ArtifactKind::UserPrompt.name(), "user_prompt");
        assert_eq!(ArtifactKind::NormalizedSpec.name(), "normalized_spec");
        assert_eq!(ArtifactKind::ScopeReport.name(), "scope_report");
    }
}
