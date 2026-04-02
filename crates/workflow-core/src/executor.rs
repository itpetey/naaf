use crate::budget::{Budget, ExecCtx, Services};
use crate::errors::{Error, StepError};
use crate::events::ExecutionEvent;
use crate::graph::{CompiledWorkflow, EdgeType, GraphNode};
use crate::route::RouteDecision;
use chrono::Utc;
use workflow_schema::state::{StateEnvelope, StateId};

pub struct Executor<S: Services> {
    workflow: CompiledWorkflow<S>,
}

struct StepResult {
    decision: RouteDecision,
    state: Option<StateEnvelope>,
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
        let initial_state_id = initial_state.id;
        ctx.trace.emit(ExecutionEvent::RunStarted {
            run_id: ctx.run_id,
            state_id: initial_state_id,
            step_name: "workflow".to_string(),
            sequence_number: ctx.next_sequence_number(),
            timestamp: Utc::now(),
        })?;

        let result = Box::pin(self.execute_from_node(
            ctx,
            initial_state,
            &self.workflow.entry_point.clone(),
        ))
        .await;

        match &result {
            Ok(final_state) => {
                ctx.trace.emit(ExecutionEvent::RunTerminated {
                    run_id: ctx.run_id,
                    state_id: final_state.id,
                    step_name: "workflow".to_string(),
                    sequence_number: ctx.next_sequence_number(),
                    timestamp: Utc::now(),
                })?;
            }
            Err(e) => {
                ctx.trace.emit(ExecutionEvent::RunFailed {
                    run_id: ctx.run_id,
                    state_id: initial_state_id,
                    step_name: "workflow".to_string(),
                    error: e.to_string(),
                    sequence_number: ctx.next_sequence_number(),
                    timestamp: Utc::now(),
                })?;
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

            ctx.trace.emit(ExecutionEvent::StepEntered {
                run_id: ctx.run_id,
                state_id: current_state.id,
                step_name: current_node_id.clone(),
                sequence_number: ctx.next_sequence_number(),
                timestamp: Utc::now(),
            })?;

            let result = self.execute_step(ctx, node, &current_state).await;

            match result {
                Ok(step_result) => {
                    if let Some(new_state) = step_result.state {
                        current_state = new_state;
                    }

                    match step_result.decision {
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
                    ctx.trace.emit(ExecutionEvent::RunFailed {
                        run_id: ctx.run_id,
                        state_id: current_state.id,
                        step_name: current_node_id.clone(),
                        error: e.to_string(),
                        sequence_number: ctx.next_sequence_number(),
                        timestamp: Utc::now(),
                    })?;
                    return Err(e);
                }
            }
        }
    }

    async fn execute_step(
        &self,
        ctx: &mut ExecCtx<S>,
        node: &GraphNode<S>,
        state: &StateEnvelope,
    ) -> Result<StepResult, Error> {
        match node {
            GraphNode::Transformer { id, transformer } => {
                let new_state = transformer.transform(ctx, state.clone())?;
                ctx.add_tokens(new_state.meta.token_count.unwrap_or(0));
                let next_node_id = self.find_next_node(id)?;
                Ok(StepResult {
                    decision: RouteDecision::next(next_node_id),
                    state: Some(new_state),
                })
            }
            GraphNode::Router { id, router } => {
                let decision = router.route(ctx, state)?;
                ctx.trace.emit(ExecutionEvent::RouteSelected {
                    run_id: ctx.run_id,
                    state_id: state.id,
                    step_name: id.clone(),
                    sequence_number: ctx.next_sequence_number(),
                    timestamp: Utc::now(),
                })?;
                Ok(StepResult {
                    decision,
                    state: None,
                })
            }
            GraphNode::Reducer { id, reducer } => {
                let inputs = vec![state.clone()];
                let merged_state = reducer.reduce(ctx, inputs)?;
                ctx.add_tokens(merged_state.meta.token_count.unwrap_or(0));
                let next_node_id = self.find_next_node(id)?;
                Ok(StepResult {
                    decision: RouteDecision::next(next_node_id),
                    state: Some(merged_state),
                })
            }
            GraphNode::Validator { .. } => Ok(StepResult {
                decision: RouteDecision::Terminal,
                state: None,
            }),
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
            run_id: ctx.run_id,
            state_id: parent_state.id,
            step_name: parent_node_id.to_string(),
            sequence_number: ctx.next_sequence_number(),
            timestamp: Utc::now(),
        })?;

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
                ctx.trace.emit(ExecutionEvent::JoinReduced {
                    run_id: ctx.run_id,
                    state_id: merged_state.id,
                    step_name: join_node_id.clone(),
                    sequence_number: ctx.next_sequence_number(),
                    timestamp: Utc::now(),
                })?;
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
            return Err(Error::from(StepError::execution(format!(
                "[run={}] Step budget exceeded: {}/{}",
                ctx.run_id, ctx.step_count, max_steps
            ))));
        }

        if let Some(max_branches) = ctx.budget.branch_limit()
            && ctx.branch_count >= max_branches
        {
            return Err(Error::from(StepError::execution(format!(
                "[run={}] Branch budget exceeded: {}/{}",
                ctx.run_id, ctx.branch_count, max_branches
            ))));
        }

        if let Some(token_budget) = ctx.budget.token_limit()
            && ctx.total_tokens >= token_budget
        {
            return Err(Error::from(StepError::execution(format!(
                "[run={}] Token budget exceeded: {}/{}",
                ctx.run_id, ctx.total_tokens, token_budget
            ))));
        }

        if let Some(time_budget_ms) = ctx.budget.time_limit_ms() {
            let elapsed_ms = ctx.start_time.elapsed().as_millis() as u64;
            if elapsed_ms >= time_budget_ms {
                return Err(Error::from(StepError::execution(format!(
                    "[run={}] Time budget exceeded: {}ms/{}ms",
                    ctx.run_id, elapsed_ms, time_budget_ms
                ))));
            }
        }

        Ok(())
    }
}
