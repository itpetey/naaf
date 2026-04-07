//! OpenSpec: workflow, workers, validators, and artifact schemas.
//!
//! **DEPRECATED - MIGRATION IN PROGRESS**
//!
//! Artifact schemas have been migrated to `naaf_schema`.
//! LLM prompts have been migrated to `naaf_llm`.
//!
//! # Migration Status
//!
//! ## Migrated
//! - All artifact types (re-exported here for backward compatibility)
//! - LLM prompt constants (now in `naaf_llm::prompts`)
//!
//! ## Legacy Components Remaining
//! - `WorkflowDefinition` - Legacy workflow structure
//! - `TransitionSpec` - Legacy transition specifications
//! - `WorkerSpec` - Legacy worker definitions
//! - `decode` module - Legacy output parsing
//!
//! # New Runtime Components
//!
//! For new development, use:
//! - `naaf_core::builder::WorkflowBuilder` - Modern workflow definition
//! - `naaf_schema` - Artifacts and state management
//! - `naaf_builtins` - Step implementations
//! - `naaf_llm::prompts` - LLM prompt templates
//!
//! See `LEGACY.md` and `MIGRATION.md` for migration details.

pub mod decode;
pub mod kind;
pub mod phase;
pub mod workers;
pub mod workflow;

// Re-export artifacts from naaf_schema for backward compatibility
pub use naaf_schema::{
    AcceptanceCriteriaSet, ConsistencyFinding, Criterion, Finding, FindingSet, FindingSeverity,
    NormalizedSpec, ProposalSkeleton, ReadinessDecision, RemediationPlan, RiskFinding, ScopeReport,
    SectionPatch,
};

pub use decode::{DecodeError, Result as DecodeResult};
pub use kind::ArtifactKind;
pub use phase::Phase;
pub use workers::{WorkerId, WorkerSpec, all_worker_specs};
pub use workflow::{TransitionSpec, WorkflowDefinition, openspec_happy_path, review_workflow};
