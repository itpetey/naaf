//! Validators for workflow systems.
//!
//! This module provides validators that check state before allowing workflow progression.

use std::marker::PhantomData;

use naaf_core::budget::{ExecCtx, Services};
use naaf_core::steps::Validator;
use naaf_schema::state::StateEnvelope;

pub struct DoneValidator<S: Services> {
    _phantom: PhantomData<S>,
}

impl<S: Services> DoneValidator<S> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<S: Services> Default for DoneValidator<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Services> Validator for DoneValidator<S> {
    type Services = S;

    fn name(&self) -> &'static str {
        "done_validator"
    }

    fn validate(
        &self,
        _ctx: &ExecCtx<Self::Services>,
        _state: &StateEnvelope,
    ) -> Result<(), naaf_core::errors::ValidationError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_services::NoopServices;
    use naaf_schema::execution_status::ExecutionStatus;
    use naaf_schema::lineage::Lineage;
    use naaf_schema::state::{RunId, StateEnvelope, StateId};
    use naaf_schema::state_kind::StateKind;

    fn make_ctx() -> ExecCtx<NoopServices> {
        ExecCtx::new(RunId::new(), NoopServices)
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
