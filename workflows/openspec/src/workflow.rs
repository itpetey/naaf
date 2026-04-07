//! OpenSpec workflow definitions.

use serde::{Deserialize, Serialize};

use crate::kind::ArtifactKind;
use crate::phase::Phase;
use crate::workers::WorkerId;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionSpec {
    pub name: String,
    pub from_phase: Phase,
    pub to_phase: Phase,
    pub consumes: Vec<ArtifactKind>,
    pub produces: ArtifactKind,
    pub worker_id: String,
    pub retry_limit: u32,
    pub timeout_secs: u64,
}

impl TransitionSpec {
    pub fn new(
        name: String,
        from_phase: Phase,
        to_phase: Phase,
        consumes: Vec<ArtifactKind>,
        produces: ArtifactKind,
        worker_id: String,
    ) -> Self {
        Self {
            name,
            from_phase,
            to_phase,
            consumes,
            produces,
            worker_id,
            retry_limit: 3,
            timeout_secs: 300,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    phases: Vec<PhaseNode>,
    transitions: Vec<TransitionSpec>,
}

impl WorkflowDefinition {
    pub fn new(id: String, name: String, description: String) -> Self {
        Self {
            id,
            name,
            description,
            phases: Vec::new(),
            transitions: Vec::new(),
        }
    }

    pub fn with_phase(mut self, node: PhaseNode) -> Self {
        self.phases.push(node);
        self
    }

    pub fn with_transition(mut self, spec: TransitionSpec) -> Self {
        self.transitions.push(spec);
        self
    }

    pub fn entry_phase(&self) -> Option<Phase> {
        self.phases.first().map(|n| n.phase)
    }

    pub fn outgoing_transitions(&self, from_phase: Phase) -> Vec<&TransitionSpec> {
        self.transitions
            .iter()
            .filter(|t| t.from_phase == from_phase)
            .collect()
    }

    pub fn is_terminal_phase(&self, phase: Phase) -> bool {
        !self.transitions.iter().any(|t| t.from_phase == phase)
    }

    pub fn phase(&self, phase: Phase) -> Option<&PhaseNode> {
        self.phases.iter().find(|n| n.phase == phase)
    }

    pub fn phases(&self) -> &[PhaseNode] {
        &self.phases
    }

    pub fn transitions(&self) -> &[TransitionSpec] {
        &self.transitions
    }
}

pub fn openspec_happy_path() -> WorkflowDefinition {
    WorkflowDefinition::new(
        "openspec-happy-path".to_string(),
        "OpenSpec Happy Path".to_string(),
        "Linear workflow: Proposed -> Normalized -> Scoped -> Planned -> Accepted".to_string(),
    )
    .with_phase(PhaseNode::consumes(
        Phase::Proposed,
        ArtifactKind::UserPrompt,
    ))
    .with_phase(PhaseNode::produces(
        Phase::Normalized,
        ArtifactKind::NormalizedSpec,
    ))
    .with_phase(PhaseNode::produces(
        Phase::Scoped,
        ArtifactKind::ScopeReport,
    ))
    .with_phase(PhaseNode::produces(
        Phase::Planned,
        ArtifactKind::ProposalSkeleton,
    ))
    .with_phase(PhaseNode::produces(
        Phase::Accepted,
        ArtifactKind::AcceptanceCriteriaSet,
    ))
    .with_transition(TransitionSpec::new(
        "normalize".to_string(),
        Phase::Proposed,
        Phase::Normalized,
        vec![ArtifactKind::UserPrompt],
        ArtifactKind::NormalizedSpec,
        WorkerId::RequestNormalizer.name().to_string(),
    ))
    .with_transition(TransitionSpec::new(
        "scope".to_string(),
        Phase::Normalized,
        Phase::Scoped,
        vec![ArtifactKind::NormalizedSpec],
        ArtifactKind::ScopeReport,
        WorkerId::ScopeAnalyst.name().to_string(),
    ))
    .with_transition(TransitionSpec::new(
        "plan".to_string(),
        Phase::Scoped,
        Phase::Planned,
        vec![ArtifactKind::NormalizedSpec, ArtifactKind::ScopeReport],
        ArtifactKind::ProposalSkeleton,
        WorkerId::ProposalSkeletonBuilder.name().to_string(),
    ))
    .with_transition(TransitionSpec::new(
        "accept".to_string(),
        Phase::Planned,
        Phase::Accepted,
        vec![ArtifactKind::NormalizedSpec, ArtifactKind::ProposalSkeleton],
        ArtifactKind::AcceptanceCriteriaSet,
        WorkerId::AcceptanceCriteriaAuthor.name().to_string(),
    ))
}

pub fn review_workflow() -> WorkflowDefinition {
    WorkflowDefinition::new(
        "openspec-review".to_string(),
        "OpenSpec Review Workflow".to_string(),
        "Workflow with review and remediation loop".to_string(),
    )
    .with_phase(PhaseNode::consumes(
        Phase::Proposed,
        ArtifactKind::UserPrompt,
    ))
    .with_phase(PhaseNode::produces(
        Phase::Normalized,
        ArtifactKind::NormalizedSpec,
    ))
    .with_phase(PhaseNode::produces(
        Phase::Scoped,
        ArtifactKind::ScopeReport,
    ))
    .with_phase(PhaseNode::produces(
        Phase::Planned,
        ArtifactKind::ProposalSkeleton,
    ))
    .with_transition(TransitionSpec::new(
        "normalize".to_string(),
        Phase::Proposed,
        Phase::Normalized,
        vec![ArtifactKind::UserPrompt],
        ArtifactKind::NormalizedSpec,
        WorkerId::RequestNormalizer.name().to_string(),
    ))
    .with_transition(TransitionSpec::new(
        "scope".to_string(),
        Phase::Normalized,
        Phase::Scoped,
        vec![ArtifactKind::NormalizedSpec],
        ArtifactKind::ScopeReport,
        WorkerId::ScopeAnalyst.name().to_string(),
    ))
    .with_transition(TransitionSpec::new(
        "plan".to_string(),
        Phase::Scoped,
        Phase::Planned,
        vec![ArtifactKind::NormalizedSpec, ArtifactKind::ScopeReport],
        ArtifactKind::ProposalSkeleton,
        WorkerId::ProposalSkeletonBuilder.name().to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_happy_path_workflow_construction() {
        let workflow = openspec_happy_path();
        assert_eq!(workflow.id, "openspec-happy-path");
        assert_eq!(workflow.name, "OpenSpec Happy Path");
    }

    #[test]
    fn test_happy_path_entry_phase() {
        let workflow = openspec_happy_path();
        assert_eq!(workflow.entry_phase(), Some(Phase::Proposed));
    }

    #[test]
    fn test_happy_path_terminal_phase() {
        let workflow = openspec_happy_path();
        assert!(workflow.is_terminal_phase(Phase::Accepted));
        assert!(!workflow.is_terminal_phase(Phase::Proposed));
    }

    #[test]
    fn test_happy_path_transitions() {
        let workflow = openspec_happy_path();
        let proposed_transitions = workflow.outgoing_transitions(Phase::Proposed);
        assert_eq!(proposed_transitions.len(), 1);
        assert_eq!(proposed_transitions[0].to_phase, Phase::Normalized);

        let normalized_transitions = workflow.outgoing_transitions(Phase::Normalized);
        assert_eq!(normalized_transitions.len(), 1);
        assert_eq!(normalized_transitions[0].to_phase, Phase::Scoped);

        let scoped_transitions = workflow.outgoing_transitions(Phase::Scoped);
        assert_eq!(scoped_transitions.len(), 1);
        assert_eq!(scoped_transitions[0].to_phase, Phase::Planned);

        let planned_transitions = workflow.outgoing_transitions(Phase::Planned);
        assert_eq!(planned_transitions.len(), 1);
        assert_eq!(planned_transitions[0].to_phase, Phase::Accepted);
    }

    #[test]
    fn test_happy_path_phase_nodes() {
        let workflow = openspec_happy_path();
        assert!(workflow.phase(Phase::Proposed).is_some());
        assert!(workflow.phase(Phase::Normalized).is_some());
        assert!(workflow.phase(Phase::Scoped).is_some());
        assert!(workflow.phase(Phase::Planned).is_some());
        assert!(workflow.phase(Phase::Accepted).is_some());
    }

    #[test]
    fn test_transition_produces_correct_artifacts() {
        let workflow = openspec_happy_path();

        let normalize_trans = workflow.outgoing_transitions(Phase::Proposed);
        assert_eq!(normalize_trans[0].produces, ArtifactKind::NormalizedSpec);

        let scope_trans = workflow.outgoing_transitions(Phase::Normalized);
        assert_eq!(scope_trans[0].produces, ArtifactKind::ScopeReport);

        let plan_trans = workflow.outgoing_transitions(Phase::Scoped);
        assert_eq!(plan_trans[0].produces, ArtifactKind::ProposalSkeleton);

        let accept_trans = workflow.outgoing_transitions(Phase::Planned);
        assert_eq!(
            accept_trans[0].produces,
            ArtifactKind::AcceptanceCriteriaSet
        );
    }
}
