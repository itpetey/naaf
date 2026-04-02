use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum StateKind {
    Proposed,
    Normalized,
    Scoped,
    Planned,
    Accepted,
    Ambiguous,
    Escalated,
    Terminal,
}
