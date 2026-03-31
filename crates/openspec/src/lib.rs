//! OpenSpec: workflow, workers, validators, and artifact schemas.

pub mod artifacts;
pub mod decode;
pub mod kind;
pub mod phase;
pub mod workers;
pub mod workflow;

pub use artifacts::{
    AcceptanceCriteriaSet, ConsistencyFinding, Criterion, Finding, FindingSet, FindingSeverity,
    NormalizedSpec, ProposalSkeleton, ReadinessDecision, RemediationPlan, RiskFinding, ScopeReport,
    SectionPatch,
};
pub use decode::{DecodeError, Result as DecodeResult};
pub use kind::ArtifactKind;
pub use phase::Phase;
pub use workers::{WorkerId, WorkerSpec, all_worker_specs};
pub use workflow::{TransitionSpec, WorkflowDefinition, openspec_happy_path};
