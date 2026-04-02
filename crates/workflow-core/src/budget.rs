use serde::{Deserialize, Serialize};

pub type StepBudget = u32;
pub type BranchBudget = u32;
pub type TokenBudget = u64;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct BudgetState {
    pub max_steps: Option<StepBudget>,
    pub max_branches: Option<BranchBudget>,
    pub token_budget: Option<TokenBudget>,
    pub time_budget_ms: Option<u64>,
}

impl BudgetState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_max_steps(mut self, steps: StepBudget) -> Self {
        self.max_steps = Some(steps);
        self
    }

    pub fn with_max_branches(mut self, branches: BranchBudget) -> Self {
        self.max_branches = Some(branches);
        self
    }

    pub fn with_token_budget(mut self, tokens: TokenBudget) -> Self {
        self.token_budget = Some(tokens);
        self
    }

    pub fn with_time_budget_ms(mut self, ms: u64) -> Self {
        self.time_budget_ms = Some(ms);
        self
    }
}

pub trait Budget {
    fn state(&self) -> &BudgetState;

    fn step_limit(&self) -> Option<StepBudget> {
        self.state().max_steps
    }

    fn branch_limit(&self) -> Option<BranchBudget> {
        self.state().max_branches
    }

    fn token_limit(&self) -> Option<TokenBudget> {
        self.state().token_budget
    }

    fn time_limit_ms(&self) -> Option<u64> {
        self.state().time_budget_ms
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BudgetImpl {
    state: BudgetState,
}

impl BudgetImpl {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_state(mut self, state: BudgetState) -> Self {
        self.state = state;
        self
    }
}

impl Budget for BudgetImpl {
    fn state(&self) -> &BudgetState {
        &self.state
    }
}

pub trait Services: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    fn call(
        &self,
        service: &str,
        request: &[u8],
    ) -> impl std::future::Future<Output = Result<Vec<u8>, Self::Error>> + Send;
}

pub struct ExecCtx<S: Services> {
    pub budget: BudgetImpl,
    pub services: S,
    pub step_count: u32,
    pub branch_count: u32,
    pub total_tokens: u64,
}

impl<S: Services> ExecCtx<S> {
    pub fn new(services: S) -> Self {
        Self {
            budget: BudgetImpl::new(),
            services,
            step_count: 0,
            branch_count: 0,
            total_tokens: 0,
        }
    }

    pub fn with_budget(mut self, budget: BudgetState) -> Self {
        self.budget = BudgetImpl::new().with_state(budget);
        self
    }

    pub fn inc_steps(&mut self) {
        self.step_count += 1;
    }

    pub fn inc_branches(&mut self) {
        self.branch_count += 1;
    }

    pub fn add_tokens(&mut self, tokens: u64) {
        self.total_tokens += tokens;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_state_default() {
        let state = BudgetState::default();
        assert!(state.max_steps.is_none());
        assert!(state.max_branches.is_none());
    }

    #[test]
    fn budget_impl_new() {
        let budget = BudgetImpl::new();
        assert!(budget.step_limit().is_none());
    }

    #[test]
    fn budget_impl_with_state() {
        let budget = BudgetImpl::new().with_state(BudgetState::new().with_max_steps(100));
        assert_eq!(budget.step_limit(), Some(100));
    }

    #[test]
    fn exec_ctx_new() {
        struct NoServices;
        impl Services for NoServices {
            type Error = std::io::Error;
            async fn call(&self, _: &str, _: &[u8]) -> Result<Vec<u8>, Self::Error> {
                Ok(vec![])
            }
        }
        let ctx = ExecCtx::new(NoServices);
        assert_eq!(ctx.step_count, 0);
    }

    #[test]
    fn exec_ctx_inc_steps() {
        struct NoServices;
        impl Services for NoServices {
            type Error = std::io::Error;
            async fn call(&self, _: &str, _: &[u8]) -> Result<Vec<u8>, Self::Error> {
                Ok(vec![])
            }
        }
        let mut ctx = ExecCtx::new(NoServices);
        ctx.inc_steps();
        ctx.inc_steps();
        assert_eq!(ctx.step_count, 2);
    }
}
