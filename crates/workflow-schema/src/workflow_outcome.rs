use serde::{Deserialize, Serialize};

/// Terminal outcomes for a workflow execution.
///
/// This represents the final state of a completed workflow in the new schema layer.
/// Note: The legacy orchestrator has a separate `Outcome` enum that will eventually
/// be replaced by this type during migration.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum WorkflowOutcome {
    Completed,
    NeedHumanClarification,
    Rejected,
    Escalated,
    Aborted,
}
