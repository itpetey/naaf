//! OpenSpec workflows, steps, and domain artifacts.
//!
//! This crate provides workflow definitions and step implementations
//! for the OpenSpec proposal authoring workflow.

pub mod accept;
pub mod artifacts;
pub mod classify_input;
pub mod decode;
pub mod kind;
pub mod llm_json;
pub mod llm_steps;
pub mod normalize;
pub mod package_steps;
pub mod phase;
pub mod plan;
pub mod prompts;
pub mod propose;
pub mod registry;
pub mod routers;
pub mod scope;
pub mod services;
pub mod terminal;
pub mod validators;

#[cfg(test)]
pub(crate) mod test_services;

pub use accept::{AcceptStep, Acceptance};
pub use artifacts::{
    AcceptanceCriteriaSet, ConsistencyFinding, ConsistencyReviewerInput, Criterion, Finding,
    FindingSet, FindingSeverity, FindingsAggregatorInput, NormalizedSpec, ProposalSkeleton,
    ReadinessDecision, ReadinessEvaluatorInput, RemediationPlan, RemediationPlannerInput,
    RiskFinding, RiskReviewerInput, ScopeReport, SectionPatch, TargetedRemediatorInput,
};
pub use classify_input::{Classification, ClassifyInput, InputClass};
pub use decode::{DecodeError, Result as DecodeResult};
pub use kind::ArtifactKind;
pub use llm_steps::{LlmAcceptanceStep, LlmNormalizeStep, LlmScopeStep, LlmSkeletonStep};
pub use normalize::{NormalizeStep, NormalizedInput};
pub use phase::Phase;
pub use plan::{EffortLevel, Plan, PlanStep};
pub use propose::{Proposal, ProposeStep};
pub use registry::{register_legacy_steps, register_workflow_steps};
pub use routers::{ConfidenceThresholdRouter, InputClassificationRouter, NeedsHumanClarification};
pub use scope::{Complexity, ScopeAnalysis, ScopeStep, ScopeType};
pub use services::{LlmServiceError, LlmServices};
pub use terminal::{EscalationTerminal, GreetingTerminal};
