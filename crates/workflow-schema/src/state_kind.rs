use serde::{Deserialize, Serialize};

/// Semantic workflow stages.
///
/// This represents where a workflow is in its lifecycle, independent of
/// execution status or terminal outcomes. These are the meaningful business
/// states that artifacts transition through.
///
/// Note: Execution status (Pending/Running/Succeeded/Failed) is tracked
/// separately in ExecutionStatus. Terminal outcomes are tracked in
/// WorkflowOutcome.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum StateKind {
    Proposed,
    Normalized,
    Scoped,
    Planned,
    Accepted,
}
