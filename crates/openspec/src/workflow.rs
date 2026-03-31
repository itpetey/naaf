//! OpenSpec workflow definitions.

use orchestrator::artifact::ArtifactKind;
use orchestrator::run::Phase;
use orchestrator::workflow::{PhaseNode, TransitionSpec, WorkflowDefinition};

use crate::workers::WorkerId;

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
