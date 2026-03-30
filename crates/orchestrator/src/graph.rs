//! Graph-based workflow representation using petgraph.

use std::collections::{HashMap, HashSet};

use petgraph::graph::NodeIndex;
use petgraph::visit::Dfs;

use crate::artifact::ArtifactKind;
use crate::run::Phase;
use crate::workflow::{TransitionSpec, WorkflowDefinition};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    MissingEntryNode,
    UnreachableNode(Phase),
    NoTerminalPhase,
    InvalidTransition { from: Phase, to: Phase },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::MissingEntryNode => write!(f, "Workflow has no entry node"),
            ValidationError::UnreachableNode(phase) => {
                write!(f, "Node {} is not reachable from entry", phase)
            }
            ValidationError::NoTerminalPhase => {
                write!(f, "Workflow has no terminal phase (cycle detected)")
            }
            ValidationError::InvalidTransition { from, to } => {
                write!(f, "Transition references invalid phase: {} -> {}", from, to)
            }
        }
    }
}

impl std::error::Error for ValidationError {}

#[derive(Debug)]
pub struct GraphWorkflow {
    graph: petgraph::Graph<Phase, TransitionSpec>,
    phase_to_index: HashMap<Phase, NodeIndex>,
    workflow: WorkflowDefinition,
}

impl GraphWorkflow {
    pub fn from_workflow(workflow: WorkflowDefinition) -> Result<Self, Vec<ValidationError>> {
        let mut graph = petgraph::Graph::new();
        let mut phase_to_index = HashMap::new();

        for phase_node in workflow.phases() {
            let idx = graph.add_node(phase_node.phase);
            phase_to_index.insert(phase_node.phase, idx);
        }

        let mut errors = Vec::new();

        for transition in workflow.transitions() {
            let from_idx = phase_to_index.get(&transition.from_phase);
            let to_idx = phase_to_index.get(&transition.to_phase);

            match (from_idx, to_idx) {
                (Some(&from), Some(&to)) => {
                    graph.add_edge(from, to, transition.clone());
                }
                (Some(_), None) => {
                    errors.push(ValidationError::InvalidTransition {
                        from: transition.from_phase,
                        to: transition.to_phase,
                    });
                }
                (None, Some(_)) => {
                    errors.push(ValidationError::InvalidTransition {
                        from: transition.from_phase,
                        to: transition.to_phase,
                    });
                }
                (None, None) => {
                    errors.push(ValidationError::InvalidTransition {
                        from: transition.from_phase,
                        to: transition.to_phase,
                    });
                }
            }
        }

        validate_graph(&graph, &phase_to_index, &mut errors);

        if errors.is_empty() {
            Ok(Self {
                graph,
                phase_to_index,
                workflow,
            })
        } else {
            Err(errors)
        }
    }

    pub fn entry_phase(&self) -> Option<Phase> {
        self.graph
            .node_indices()
            .min_by_key(|&idx| idx.index())
            .map(|idx| self.graph[idx])
    }

    pub fn terminal_phases(&self) -> Vec<Phase> {
        self.graph
            .node_indices()
            .filter(|&idx| self.graph.edges(idx).next().is_none())
            .map(|idx| self.graph[idx])
            .collect()
    }

    pub fn node_index(&self, phase: Phase) -> Option<NodeIndex> {
        self.phase_to_index.get(&phase).copied()
    }

    pub fn executable_transitions(
        &self,
        current_phase: Phase,
        available_artifacts: Option<&HashSet<ArtifactKind>>,
    ) -> Vec<&TransitionSpec> {
        let Some(from_idx) = self.node_index(current_phase) else {
            return Vec::new();
        };

        let outgoing = self.graph.edges(from_idx);

        match available_artifacts {
            Some(artifacts) => outgoing
                .filter_map(|edge| {
                    let spec = edge.weight();
                    let has_required = spec.consumes.iter().all(|kind| artifacts.contains(kind));
                    if has_required { Some(spec) } else { None }
                })
                .collect(),
            None => outgoing.map(|edge| edge.weight()).collect(),
        }
    }

    pub fn workflow(&self) -> &WorkflowDefinition {
        &self.workflow
    }
}

fn validate_graph(
    graph: &petgraph::Graph<Phase, TransitionSpec>,
    _phase_to_index: &HashMap<Phase, NodeIndex>,
    errors: &mut Vec<ValidationError>,
) {
    if graph.node_count() == 0 {
        errors.push(ValidationError::MissingEntryNode);
        return;
    }

    let entry_idx = graph
        .node_indices()
        .min_by_key(|&idx| idx.index())
        .expect("node_count > 0");

    let mut visited = HashSet::new();
    let mut dfs = Dfs::new(graph, entry_idx);
    while let Some(visited_idx) = dfs.next(graph) {
        visited.insert(visited_idx);
    }

    for idx in graph.node_indices() {
        if !visited.contains(&idx) {
            errors.push(ValidationError::UnreachableNode(graph[idx]));
        }
    }

    let has_terminal = graph
        .node_indices()
        .any(|idx| graph.edges(idx).next().is_none());
    if !has_terminal {
        errors.push(ValidationError::NoTerminalPhase);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::{PhaseNode, TransitionSpec};

    fn create_test_workflow() -> WorkflowDefinition {
        WorkflowDefinition::new(
            "test-1".to_string(),
            "Test Workflow".to_string(),
            "A test workflow".to_string(),
        )
        .with_phase(PhaseNode::milestone(Phase::Proposed))
        .with_phase(PhaseNode::milestone(Phase::ReadyForPlanning))
        .with_phase(PhaseNode::milestone(Phase::ReadyForImplementation))
        .with_phase(PhaseNode::milestone(Phase::Terminal))
        .with_transition(TransitionSpec::new(
            "plan".to_string(),
            Phase::Proposed,
            Phase::ReadyForPlanning,
            vec![],
            ArtifactKind::TaskPlan,
            "test-worker".to_string(),
        ))
        .with_transition(TransitionSpec::new(
            "implement".to_string(),
            Phase::ReadyForPlanning,
            Phase::ReadyForImplementation,
            vec![ArtifactKind::TaskPlan],
            ArtifactKind::CandidatePatch,
            "test-worker".to_string(),
        ))
        .with_transition(TransitionSpec::new(
            "finish".to_string(),
            Phase::ReadyForImplementation,
            Phase::Terminal,
            vec![ArtifactKind::CandidatePatch],
            ArtifactKind::DeliveryBundle,
            "test-worker".to_string(),
        ))
    }

    #[test]
    fn test_graph_workflow_construction() {
        let workflow = create_test_workflow();
        let graph = GraphWorkflow::from_workflow(workflow).expect("valid workflow");
        assert!(graph.entry_phase().is_some());
    }

    #[test]
    fn test_entry_phase() {
        let workflow = create_test_workflow();
        let graph = GraphWorkflow::from_workflow(workflow).expect("valid workflow");
        assert_eq!(graph.entry_phase(), Some(Phase::Proposed));
    }

    #[test]
    fn test_terminal_phases() {
        let workflow = create_test_workflow();
        let graph = GraphWorkflow::from_workflow(workflow).expect("valid workflow");
        let terminals = graph.terminal_phases();
        assert!(terminals.contains(&Phase::Terminal));
    }

    #[test]
    fn test_missing_entry_node() {
        let workflow = WorkflowDefinition::new(
            "empty".to_string(),
            "Empty".to_string(),
            "No phases".to_string(),
        );
        let result = GraphWorkflow::from_workflow(workflow);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.contains(&ValidationError::MissingEntryNode));
    }

    #[test]
    fn test_unreachable_node() {
        let workflow = WorkflowDefinition::new(
            "unreachable".to_string(),
            "Unreachable".to_string(),
            "Has unreachable".to_string(),
        )
        .with_phase(PhaseNode::milestone(Phase::Proposed))
        .with_phase(PhaseNode::milestone(Phase::Terminal))
        .with_phase(PhaseNode::milestone(Phase::ReadyForReview))
        .with_transition(TransitionSpec::new(
            "go".to_string(),
            Phase::Proposed,
            Phase::Terminal,
            vec![],
            ArtifactKind::DeliveryBundle,
            "worker".to_string(),
        ));
        let result = GraphWorkflow::from_workflow(workflow);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, ValidationError::UnreachableNode(_)))
        );
    }

    #[test]
    fn test_no_terminal_phase() {
        let workflow = WorkflowDefinition::new(
            "cycle".to_string(),
            "Cycle".to_string(),
            "Has cycle".to_string(),
        )
        .with_phase(PhaseNode::milestone(Phase::Proposed))
        .with_phase(PhaseNode::milestone(Phase::ReadyForPlanning))
        .with_transition(TransitionSpec::new(
            "loop".to_string(),
            Phase::Proposed,
            Phase::ReadyForPlanning,
            vec![],
            ArtifactKind::TaskPlan,
            "worker".to_string(),
        ))
        .with_transition(TransitionSpec::new(
            "loopback".to_string(),
            Phase::ReadyForPlanning,
            Phase::Proposed,
            vec![ArtifactKind::TaskPlan],
            ArtifactKind::TaskPlan,
            "worker".to_string(),
        ));
        let result = GraphWorkflow::from_workflow(workflow);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.contains(&ValidationError::NoTerminalPhase));
    }

    #[test]
    fn test_executable_transitions() {
        let workflow = create_test_workflow();
        let graph = GraphWorkflow::from_workflow(workflow).expect("valid workflow");
        let exec = graph.executable_transitions(Phase::Proposed, None);
        assert_eq!(exec.len(), 1);
        assert_eq!(exec[0].from_phase, Phase::Proposed);
    }

    #[test]
    fn test_artifact_filtering() {
        let workflow = create_test_workflow();
        let graph = GraphWorkflow::from_workflow(workflow).expect("valid workflow");
        let mut artifacts = HashSet::new();
        artifacts.insert(ArtifactKind::TaskPlan);
        let exec = graph.executable_transitions(Phase::ReadyForPlanning, Some(&artifacts));
        assert_eq!(exec.len(), 1);
        let exec_no_artifacts = graph.executable_transitions(Phase::ReadyForPlanning, None);
        assert_eq!(exec_no_artifacts.len(), 1);
    }

    #[test]
    fn test_unknown_phase_returns_empty() {
        let workflow = create_test_workflow();
        let graph = GraphWorkflow::from_workflow(workflow).expect("valid workflow");
        let exec = graph.executable_transitions(Phase::Accepted, None);
        assert!(exec.is_empty());
    }
}
