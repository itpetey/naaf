//! Validators for workflow systems.
//!
//! This module provides validators that check state before allowing workflow progression.

use workflow_core::budget::{DummyServices, ExecCtx};
use workflow_core::steps::Validator;
use workflow_schema::state::StateEnvelope;

pub struct DoneValidator;

impl DoneValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DoneValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Validator for DoneValidator {
    type Services = DummyServices;

    fn name(&self) -> &'static str {
        "done_validator"
    }

    fn validate(
        &self,
        _ctx: &ExecCtx<Self::Services>,
        _state: &StateEnvelope,
    ) -> Result<(), workflow_core::errors::ValidationError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use workflow_core::budget::DummyServices;
    use workflow_schema::execution_status::ExecutionStatus;
    use workflow_schema::lineage::Lineage;
    use workflow_schema::state::{RunId, StateEnvelope, StateId};
    use workflow_schema::state_kind::StateKind;

    fn make_ctx() -> ExecCtx<DummyServices> {
        ExecCtx::new(RunId::new(), DummyServices)
    }

    fn make_state() -> StateEnvelope {
        StateEnvelope::new(
            StateId::new(),
            RunId::new(),
            StateKind::Proposed,
            Lineage::new(None, None, ExecutionStatus::Pending),
        )
    }

    #[test]
    fn test_done_validator_accepts_any_state() {
        let validator = DoneValidator::new();
        let ctx = make_ctx();
        let state = make_state();

        let result = validator.validate(&ctx, &state);
        assert!(result.is_ok());
    }
}
