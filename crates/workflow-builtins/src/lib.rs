//! Built-in workflow steps for common patterns.
//!
//! This crate provides reusable transformers, routers, and terminal handlers
//! for workflow systems, including input classification, routing logic,
//! and escalation handling.

pub mod accept;
pub mod clarify;
pub mod classify_input;
pub mod keys;
pub mod normalize;
pub mod plan;
pub mod propose;
pub mod reducers;
pub mod routers;
pub mod scope;
pub mod terminal;
pub mod validators;
pub mod workflows;

pub use accept::{AcceptStep, Acceptance};
pub use classify_input::{Classification, ClassifyInput, InputClass};
pub use keys::DraftRequestKeys;
pub use normalize::{NormalizeStep, NormalizedInput};
pub use plan::{EffortLevel, Plan, PlanStep};
pub use propose::{Proposal, ProposeStep};
pub use routers::{ConfidenceThresholdRouter, InputClassificationRouter, NeedsHumanClarification};
pub use scope::{Complexity, ScopeAnalysis, ScopeStep, ScopeType};
pub use terminal::{EscalationTerminal, GreetingTerminal};
pub use validators::DoneValidator;
pub use workflows::draft_request_workflow;

// Backward compatibility alias
#[deprecated(note = "Use DraftRequestKeys instead")]
pub use keys::ClassificationKeys;
