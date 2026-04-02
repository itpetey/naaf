use crate::budget::{Budget, ExecCtx, Services};
use crate::errors::{Error, StepError};
use crate::events::ExecutionEvent;
use crate::graph::{CompiledWorkflow, EdgeType, GraphNode};
use crate::route::RouteDecision;
use workflow_schema::state::{StateEnvelope, StateId};

pub struct Executor<S: Services> {
    workflow: CompiledWorkflow<S>,
}

impl<S: Services> Executor<S> {
    pub fn new(workflow: CompiledWorkflow<S>) -> Result<Self, Error> {
        workflow.validate()?;
        Ok(Self { workflow })
    }

    pub async fn execute(
        &self,
        ctx: &mut ExecCtx<S>,
        initial_state: StateEnvelope,
    ) -> Result<StateEnvelope, Error> {
        ctx.trace.emit(ExecutionEvent::WorkflowStarted {
            run_id: ctx.run_id,
            initial_state: initial_state.id,
        });

        let result = Box::pin(self.execute_from_node(
            ctx,
            initial_state,
            &self.workflow.entry_point.clone(),
        ))
        .await;

        match &result {
            Ok(final_state) => {
                ctx.trace.emit(ExecutionEvent::WorkflowCompleted {
                    run_id: ctx.run_id,
                    final_state: final_state.id,
                    final_kind: final_state.kind,
                });
            }
            Err(e) => {
                ctx.trace.emit(ExecutionEvent::WorkflowFailed {
                    run_id: ctx.run_id,
                    error: e.to_string(),
                });
            }
        }

        result
    }

    async fn execute_from_node(
        &'_ self,
        ctx: &'_ mut ExecCtx<S>,
        mut current_state: StateEnvelope,
        node_id: &str,
    ) -> Result<StateEnvelope, Error> {
        let mut current_node_id = node_id.to_string();

        loop {
            if ctx.cancel.is_cancelled() {
                return Err(Error::from(StepError::execution(format!(
                    "[run={}, node={}] Workflow cancelled",
                    ctx.run_id, current_node_id
                ))));
            }

            self.check_budget(ctx)?;

            let node = self.workflow.get_node(&current_node_id).ok_or_else(|| {
                StepError::execution(format!(
                    "[run={}] Node '{}' not found in workflow graph",
                    ctx.run_id, current_node_id
                ))
            })?;

            ctx.inc_steps();

            ctx.trace.emit(ExecutionEvent::StepStarted {
                step_id: current_node_id.clone(),
                state_id: current_state.id,
            });

            let result = self.execute_step(ctx, node, &current_state).await;

            match result {
                Ok(route_decision) => {
                    ctx.trace.emit(ExecutionEvent::StepCompleted {
                        step_id: current_node_id.clone(),
                        state_id: current_state.id,
                    });

                    match route_decision {
                        RouteDecision::Terminal => {
                            return Ok(current_state);
                        }
                        RouteDecision::Next(next_node_id) => {
                            if next_node_id.is_empty() {
                                return Ok(current_state);
                            }
                            current_node_id = next_node_id;
                        }
                        RouteDecision::Branch(branch_node_ids) => {
                            current_state = Box::pin(self.execute_branches(
                                ctx,
                                &current_node_id,
                                &branch_node_ids,
                                &current_state,
                            ))
                            .await?;

                            let join_node_id = self.find_join_node(&current_node_id)?;
                            if join_node_id.is_empty() {
                                return Ok(current_state);
                            }
                            current_node_id = join_node_id;
                        }
                    }
                }
                Err(e) => {
                    ctx.trace.emit(ExecutionEvent::StepFailed {
                        step_id: current_node_id.clone(),
                        error: e.to_string(),
                    });
                    return Err(Error::from(e));
                }
            }
        }
    }

    async fn execute_step(
        &self,
        ctx: &mut ExecCtx<S>,
        node: &GraphNode<S>,
        state: &StateEnvelope,
    ) -> Result<RouteDecision, StepError> {
        match node {
            GraphNode::Transformer { id, transformer } => {
                let new_state = transformer.transform(ctx, state.clone())?;
                ctx.add_tokens(new_state.meta.token_count.unwrap_or(0));
                let next_node_id = self.find_next_node(id)?;
                Ok(RouteDecision::next(next_node_id))
            }
            GraphNode::Router { router, .. } => router.route(ctx, state),
            GraphNode::Reducer { id, reducer } => {
                let inputs = vec![state.clone()];
                let merged_state = reducer.reduce(ctx, inputs)?;
                ctx.add_tokens(merged_state.meta.token_count.unwrap_or(0));
                let next_node_id = self.find_next_node(id)?;
                Ok(RouteDecision::next(next_node_id))
            }
            GraphNode::Validator { .. } => Ok(RouteDecision::Terminal),
        }
    }

    async fn execute_branches(
        &'_ self,
        ctx: &'_ mut ExecCtx<S>,
        parent_node_id: &str,
        branch_node_ids: &[String],
        parent_state: &StateEnvelope,
    ) -> Result<StateEnvelope, Error> {
        ctx.inc_branches();
        ctx.trace.emit(ExecutionEvent::BranchStarted {
            branch_count: branch_node_ids.len() as u32,
        });

        let mut branch_states = Vec::new();

        for branch_node_id in branch_node_ids {
            let branch_state = StateEnvelope::new(
                StateId::new(),
                ctx.run_id,
                parent_state.kind,
                parent_state.lineage.clone(),
            );

            let branch_result =
                Box::pin(self.execute_from_node(ctx, branch_state, branch_node_id)).await?;
            branch_states.push(branch_result);
        }

        let join_node_id = self.find_join_node(parent_node_id)?;
        let join_node = self.workflow.get_node(&join_node_id).ok_or_else(|| {
            StepError::execution(format!(
                "[run={}] Join node '{}' not found",
                ctx.run_id, join_node_id
            ))
        })?;

        match join_node {
            GraphNode::Reducer { reducer, .. } => {
                let merged_state = reducer.reduce(ctx, branch_states)?;
                ctx.trace.emit(ExecutionEvent::BranchCompleted {
                    merged_state: merged_state.id,
                });
                Ok(merged_state)
            }
            _ => Err(Error::from(StepError::execution(format!(
                "[run={}] Join node '{}' must be a Reducer, found {:?}",
                ctx.run_id,
                join_node_id,
                join_node.id()
            )))),
        }
    }

    fn find_join_node(&self, branch_node_id: &str) -> Result<String, StepError> {
        for edge in &self.workflow.edges {
            if edge.source == branch_node_id && edge.edge_type == EdgeType::Join {
                return Ok(edge.target.clone());
            }
        }
        Err(StepError::execution(format!(
            "[node={}] No join node found for branch node",
            branch_node_id
        )))
    }

    fn find_next_node(&self, current_node_id: &str) -> Result<String, StepError> {
        for edge in &self.workflow.edges {
            if edge.source == current_node_id && edge.edge_type == EdgeType::Normal {
                return Ok(edge.target.clone());
            }
        }
        Ok(String::new())
    }

    fn check_budget(&self, ctx: &ExecCtx<S>) -> Result<(), Error> {
        if let Some(max_steps) = ctx.budget.step_limit()
            && ctx.step_count >= max_steps
        {
            ctx.trace.emit(ExecutionEvent::BudgetExceeded {
                limit: "steps".to_string(),
                current: ctx.step_count as u64,
                max: max_steps as u64,
            });
            return Err(Error::from(StepError::execution(format!(
                "[run={}] Step budget exceeded: {}/{}",
                ctx.run_id, ctx.step_count, max_steps
            ))));
        }

        if let Some(max_branches) = ctx.budget.branch_limit()
            && ctx.branch_count >= max_branches
        {
            ctx.trace.emit(ExecutionEvent::BudgetExceeded {
                limit: "branches".to_string(),
                current: ctx.branch_count as u64,
                max: max_branches as u64,
            });
            return Err(Error::from(StepError::execution(format!(
                "[run={}] Branch budget exceeded: {}/{}",
                ctx.run_id, ctx.branch_count, max_branches
            ))));
        }

        if let Some(token_budget) = ctx.budget.token_limit()
            && ctx.total_tokens >= token_budget
        {
            ctx.trace.emit(ExecutionEvent::BudgetExceeded {
                limit: "tokens".to_string(),
                current: ctx.total_tokens,
                max: token_budget,
            });
            return Err(Error::from(StepError::execution(format!(
                "[run={}] Token budget exceeded: {}/{}",
                ctx.run_id, ctx.total_tokens, token_budget
            ))));
        }

        if let Some(time_budget_ms) = ctx.budget.time_limit_ms() {
            let elapsed_ms = ctx.start_time.elapsed().as_millis() as u64;
            if elapsed_ms >= time_budget_ms {
                ctx.trace.emit(ExecutionEvent::BudgetExceeded {
                    limit: "time".to_string(),
                    current: elapsed_ms,
                    max: time_budget_ms,
                });
                return Err(Error::from(StepError::execution(format!(
                    "[run={}] Time budget exceeded: {}ms/{}ms",
                    ctx.run_id, elapsed_ms, time_budget_ms
                ))));
            }
        }

        Ok(())
    }
}
