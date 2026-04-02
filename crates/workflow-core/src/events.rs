use serde::{Deserialize, Serialize};
use workflow_schema::state::{RunId, StateId};
use workflow_schema::state_kind::StateKind;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum ExecutionEvent {
    StepStarted {
        step_id: String,
        state_id: StateId,
    },
    StepCompleted {
        step_id: String,
        state_id: StateId,
    },
    StepFailed {
        step_id: String,
        error: String,
    },
    BranchStarted {
        branch_count: u32,
    },
    BranchCompleted {
        merged_state: StateId,
    },
    WorkflowStarted {
        run_id: RunId,
        initial_state: StateId,
    },
    WorkflowCompleted {
        run_id: RunId,
        final_state: StateId,
        final_kind: StateKind,
    },
    WorkflowFailed {
        run_id: RunId,
        error: String,
    },
    BudgetExceeded {
        limit: String,
        current: u64,
        max: u64,
    },
}

pub trait EventSink: Send + Sync {
    fn emit(&self, event: ExecutionEvent);
}

#[derive(Clone, Default)]
pub struct NullEventSink;

impl EventSink for NullEventSink {
    fn emit(&self, _event: ExecutionEvent) {}
}
