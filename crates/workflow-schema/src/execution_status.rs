use serde::{Deserialize, Serialize};

/// Runtime execution status for a state.
///
/// This tracks whether a state is pending, currently running, or has completed
/// with success or failure. This is orthogonal to the semantic StateKind and
/// represents the runtime execution lifecycle.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum ExecutionStatus {
    #[default]
    Pending,
    Running,
    Succeeded,
    Failed,
}
