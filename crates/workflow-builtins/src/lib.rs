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
pub mod reducers;
pub mod routers;
pub mod terminal;
pub mod validators;

pub use classify_input::{Classification, ClassifyInput, InputClass};
pub use keys::ClassificationKeys;
pub use routers::{ConfidenceThresholdRouter, NeedsHumanClarification};
pub use terminal::{EscalationTerminal, GreetingTerminal};
