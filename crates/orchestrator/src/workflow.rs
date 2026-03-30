//! Workflow graph definition and execution.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::artifact::ArtifactKind;
use crate::run::Phase;

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
    #[serde(skip)]
    phase_index: HashMap<Phase, usize>,
}

impl WorkflowDefinition {
    pub fn new(id: String, name: String, description: String) -> Self {
        Self {
            id,
            name,
            description,
            phases: Vec::new(),
            transitions: Vec::new(),
            phase_index: HashMap::new(),
        }
    }

    pub fn with_phase(mut self, node: PhaseNode) -> Self {
        let idx = self.phases.len();
        self.phase_index.insert(node.phase, idx);
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
        self.phase_index
            .get(&phase)
            .and_then(|&idx| self.phases.get(idx))
    }

    pub fn phases(&self) -> &[PhaseNode] {
        &self.phases
    }

    pub fn transitions(&self) -> &[TransitionSpec] {
        &self.transitions
    }
}

pub trait ExecutionEngine: Send + Sync {
    fn execute_transition(
        &self,
        run: &mut crate::run::Run,
        spec: &TransitionSpec,
    ) -> Result<crate::artifact::Artifact, EngineError>;

    fn can_execute(&self, run: &crate::run::Run, spec: &TransitionSpec) -> bool;
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

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
