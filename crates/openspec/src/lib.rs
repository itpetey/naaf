//! OpenSpec: workflow, workers, validators, and artifact schemas.

pub mod artifacts;
pub mod workers;
pub mod workflow;

pub use artifacts::{
    AcceptanceCriteriaSet, Criterion, NormalizedSpec, ProposalSkeleton, ScopeReport,
};
pub use workers::{WorkerId, WorkerSpec, all_worker_specs};
pub use workflow::openspec_happy_path;
