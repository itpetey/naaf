use serde::{Deserialize, Serialize};

use crate::execution_status::ExecutionStatus;
use crate::state::StateId;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Lineage {
    pub parent_state_id: Option<StateId>,
    pub transition_name: Option<String>,
    pub execution_status: ExecutionStatus,
}

impl Lineage {
    pub fn new(
        parent_state_id: Option<StateId>,
        transition_name: Option<String>,
        execution_status: ExecutionStatus,
    ) -> Self {
        Self {
            parent_state_id,
            transition_name,
            execution_status,
        }
    }
}
