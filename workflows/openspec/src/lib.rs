//! OpenSpec workflows, steps, prompts, and domain artifacts.

pub mod accept;
pub mod artifacts;
pub mod classify_input;
pub mod decode;
pub mod keys;
pub mod kind;
pub mod llm_steps;
pub mod mock_llm;
pub mod normalize;
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
pub mod workers;
pub mod workflow;
pub mod workflows;

pub use accept::{AcceptStep, Acceptance};
pub use artifacts::{
    AcceptanceCriteriaSet, ConsistencyFinding, ConsistencyReviewerInput, Criterion, Finding,
    FindingSet, FindingSeverity, FindingsAggregatorInput, NormalizedSpec, ProposalSkeleton,
    ReadinessDecision, ReadinessEvaluatorInput, RemediationPlan, RemediationPlannerInput,
    RiskFinding, RiskReviewerInput, ScopeReport, SectionPatch, TargetedRemediatorInput,
};
pub use classify_input::{Classification, ClassifyInput, InputClass};
pub use decode::{DecodeError, Result as DecodeResult};
pub use keys::DraftRequestKeys;
pub use kind::ArtifactKind;
pub use llm_steps::{LlmAcceptanceStep, LlmNormalizeStep, LlmScopeStep, LlmSkeletonStep};
pub use mock_llm::MockLlmServices;
pub use normalize::{NormalizeStep, NormalizedInput};
pub use phase::Phase;
pub use plan::{EffortLevel, Plan, PlanStep};
pub use propose::{Proposal, ProposeStep};
pub use registry::{register_legacy_steps, register_workflow_steps};
pub use routers::{ConfidenceThresholdRouter, InputClassificationRouter, NeedsHumanClarification};
pub use scope::{Complexity, ScopeAnalysis, ScopeStep, ScopeType};
pub use services::{LlmServiceError, LlmServices};
pub use terminal::{EscalationTerminal, GreetingTerminal};
pub use validators::DoneValidator;
pub use workers::{WorkerId, WorkerSpec, all_worker_specs};
pub use workflow::{TransitionSpec, WorkflowDefinition, openspec_happy_path, review_workflow};
pub use workflows::{draft_request_workflow, openspec_happy_path_llm};

// Backward compatibility alias
#[deprecated(note = "Use DraftRequestKeys instead")]
pub use keys::ClassificationKeys;
