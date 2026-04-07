use crate::budget::{Budget, ExecCtx, Services};
use crate::errors::{Error, StepError};
use crate::events::ExecutionEvent;
use crate::graph::{CompiledWorkflow, EdgeType, GraphNode};
use crate::route::RouteDecision;
use chrono::Utc;
use naaf_schema::state::{StateEnvelope, StateId};
use naaf_schema::{execution_status::ExecutionStatus, lineage::Lineage, meta::StateMeta};

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
        ctx.remember_state(&initial_state);

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
        ctx.remember_state(&current_state);

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
                    ctx.remember_state(&current_state);

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
                            let next_node_id = self.find_next_node(&join_node_id)?;
                            if next_node_id.is_empty() {
                                return Ok(current_state);
                            }
                            current_node_id = next_node_id;
                        }
                    }
                }
                Err(e) => return Err(e),
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
            GraphNode::Validator { id, validator } => match validator.validate(ctx, state) {
                Ok(()) => {
                    ctx.trace.emit(ExecutionEvent::ValidatorPassed {
                        run_id: ctx.run_id,
                        state_id: state.id,
                        step_name: id.clone(),
                        sequence_number: ctx.next_sequence_number(),
                        timestamp: Utc::now(),
                    })?;
                    Ok(StepResult {
                        decision: RouteDecision::Terminal,
                        state: None,
                    })
                }
                Err(error) => {
                    ctx.trace.emit(ExecutionEvent::ValidatorFailed {
                        run_id: ctx.run_id,
                        state_id: state.id,
                        step_name: id.clone(),
                        sequence_number: ctx.next_sequence_number(),
                        timestamp: Utc::now(),
                    })?;
                    Err(Error::Validation(error))
                }
            },
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
            let mut branch_state = parent_state.clone();
            branch_state.id = StateId::new();
            branch_state.meta = StateMeta::now();
            branch_state.lineage = Lineage::new(
                Some(parent_state.id),
                Some(parent_node_id.to_string()),
                ExecutionStatus::Pending,
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

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use naaf_schema::artifacts::{ArtifactKey, ArtifactValue};
    use naaf_schema::execution_status::ExecutionStatus;
    use naaf_schema::lineage::Lineage;
    use naaf_schema::state_kind::StateKind;

    use crate::budget::DummyServices;
    use crate::events::{EventResult, TraceSink};
    use crate::graph::{CompiledWorkflow, GraphEdge, GraphNode};
    use crate::steps::{
        BoxedReducer, BoxedRouter, BoxedTransformer, BoxedValidator, Reducer, Router, Transformer,
        Validator,
    };

    #[derive(Clone, Default)]
    struct RecordingTraceSink {
        events: Arc<Mutex<Vec<ExecutionEvent>>>,
    }

    impl RecordingTraceSink {
        fn new() -> Self {
            Self::default()
        }

        fn events(&self) -> Vec<ExecutionEvent> {
            self.events.lock().unwrap().clone()
        }
    }

    impl TraceSink for RecordingTraceSink {
        fn emit(&self, event: ExecutionEvent) -> EventResult {
            self.events.lock().unwrap().push(event);
            Ok(())
        }
    }

    struct BranchRouter;

    impl Router for BranchRouter {
        type Services = DummyServices;

        fn name(&self) -> &'static str {
            "branch_router"
        }

        fn route(
            &self,
            _ctx: &mut ExecCtx<Self::Services>,
            _state: &StateEnvelope,
        ) -> Result<RouteDecision, StepError> {
            Ok(RouteDecision::branch(vec![
                "left".to_string(),
                "right".to_string(),
            ]))
        }
    }

    struct PassthroughTransformer;

    impl Transformer for PassthroughTransformer {
        type Services = DummyServices;

        fn name(&self) -> &'static str {
            "passthrough"
        }

        fn transform(
            &self,
            _ctx: &mut ExecCtx<Self::Services>,
            input: StateEnvelope,
        ) -> Result<StateEnvelope, StepError> {
            Ok(input)
        }
    }

    struct FailingTransformer;

    impl Transformer for FailingTransformer {
        type Services = DummyServices;

        fn name(&self) -> &'static str {
            "failing"
        }

        fn transform(
            &self,
            _ctx: &mut ExecCtx<Self::Services>,
            _input: StateEnvelope,
        ) -> Result<StateEnvelope, StepError> {
            Err(StepError::execution("boom"))
        }
    }

    struct ArtifactCheckingTransformer;

    impl Transformer for ArtifactCheckingTransformer {
        type Services = DummyServices;

        fn name(&self) -> &'static str {
            "artifact_checking"
        }

        fn transform(
            &self,
            _ctx: &mut ExecCtx<Self::Services>,
            input: StateEnvelope,
        ) -> Result<StateEnvelope, StepError> {
            let value = input
                .artifacts
                .get(&ArtifactKey::new("input"))
                .and_then(|value| value.as_text())
                .ok_or_else(|| StepError::execution("missing input artifact in branch"))?;
            if value != "branch me" {
                return Err(StepError::execution("branch received wrong input artifact"));
            }
            Ok(input)
        }
    }

    struct CountingReducer {
        calls: Arc<AtomicUsize>,
    }

    impl Reducer for CountingReducer {
        type Services = DummyServices;

        fn name(&self) -> &'static str {
            "counting_reducer"
        }

        fn reduce(
            &self,
            _ctx: &mut ExecCtx<Self::Services>,
            inputs: Vec<StateEnvelope>,
        ) -> Result<StateEnvelope, StepError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            inputs
                .into_iter()
                .next()
                .ok_or_else(|| StepError::execution("missing branch states"))
        }
    }

    struct DoneValidator;

    impl Validator for DoneValidator {
        type Services = DummyServices;

        fn name(&self) -> &'static str {
            "done"
        }

        fn validate(
            &self,
            _ctx: &ExecCtx<Self::Services>,
            _state: &StateEnvelope,
        ) -> Result<(), crate::errors::ValidationError> {
            Ok(())
        }
    }

    struct FailingValidator;

    impl Validator for FailingValidator {
        type Services = DummyServices;

        fn name(&self) -> &'static str {
            "failing_validator"
        }

        fn validate(
            &self,
            _ctx: &ExecCtx<Self::Services>,
            _state: &StateEnvelope,
        ) -> Result<(), crate::errors::ValidationError> {
            Err(crate::errors::ValidationError::validator(
                "failing_validator",
                "not valid",
            ))
        }
    }

    fn make_state(run_id: naaf_schema::state::RunId) -> StateEnvelope {
        StateEnvelope::new(
            StateId::new(),
            run_id,
            StateKind::Proposed,
            Lineage::new(None, None, ExecutionStatus::Pending),
        )
    }

    fn make_state_with_input(run_id: naaf_schema::state::RunId, input: &str) -> StateEnvelope {
        let mut state = make_state(run_id);
        state.artifacts.insert(
            ArtifactKey::new("input"),
            ArtifactValue::text(input.to_string()),
        );
        state
    }

    #[tokio::test]
    async fn branch_join_reducer_runs_once() {
        let reducer_calls = Arc::new(AtomicUsize::new(0));

        let mut workflow = CompiledWorkflow::new("branching", "route");
        workflow.add_node(GraphNode::router("route", BoxedRouter::new(BranchRouter)));
        workflow.add_node(GraphNode::transformer(
            "left",
            BoxedTransformer::new(PassthroughTransformer),
        ));
        workflow.add_node(GraphNode::transformer(
            "right",
            BoxedTransformer::new(PassthroughTransformer),
        ));
        workflow.add_node(GraphNode::reducer(
            "join",
            BoxedReducer::new(CountingReducer {
                calls: reducer_calls.clone(),
            }),
        ));
        workflow.add_node(GraphNode::validator(
            "done",
            BoxedValidator::new(DoneValidator),
        ));
        workflow.add_edge(GraphEdge::conditional("route", "left"));
        workflow.add_edge(GraphEdge::conditional("route", "right"));
        workflow.add_edge(GraphEdge::join("route", "join"));
        workflow.add_edge(GraphEdge::new("join", "done"));

        let executor = Executor::new(workflow).unwrap();
        let trace = RecordingTraceSink::new();
        let run_id = naaf_schema::state::RunId::new();
        let mut ctx = ExecCtx::new(run_id, DummyServices).with_trace(Box::new(trace));

        let _final_state = executor
            .execute(&mut ctx, make_state(run_id))
            .await
            .unwrap();

        assert_eq!(reducer_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn execution_failure_emits_one_run_failed_event() {
        let mut workflow = CompiledWorkflow::new("failing", "start");
        workflow.add_node(GraphNode::transformer(
            "start",
            BoxedTransformer::new(FailingTransformer),
        ));

        let executor = Executor::new(workflow).unwrap();
        let trace = RecordingTraceSink::new();
        let run_id = naaf_schema::state::RunId::new();
        let mut ctx = ExecCtx::new(run_id, DummyServices).with_trace(Box::new(trace.clone()));

        let error = executor
            .execute(&mut ctx, make_state(run_id))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("boom"));

        let run_failed_count = trace
            .events()
            .into_iter()
            .filter(|event| matches!(event, ExecutionEvent::RunFailed { .. }))
            .count();
        assert_eq!(run_failed_count, 1);
    }

    #[tokio::test]
    async fn branches_preserve_parent_artifacts() {
        let mut workflow = CompiledWorkflow::new("branching", "route");
        workflow.add_node(GraphNode::router("route", BoxedRouter::new(BranchRouter)));
        workflow.add_node(GraphNode::transformer(
            "left",
            BoxedTransformer::new(ArtifactCheckingTransformer),
        ));
        workflow.add_node(GraphNode::transformer(
            "right",
            BoxedTransformer::new(ArtifactCheckingTransformer),
        ));
        workflow.add_node(GraphNode::reducer(
            "join",
            BoxedReducer::new(CountingReducer {
                calls: Arc::new(AtomicUsize::new(0)),
            }),
        ));
        workflow.add_node(GraphNode::validator(
            "done",
            BoxedValidator::new(DoneValidator),
        ));
        workflow.add_edge(GraphEdge::conditional("route", "left"));
        workflow.add_edge(GraphEdge::conditional("route", "right"));
        workflow.add_edge(GraphEdge::join("route", "join"));
        workflow.add_edge(GraphEdge::new("join", "done"));

        let executor = Executor::new(workflow).unwrap();
        let run_id = naaf_schema::state::RunId::new();
        let mut ctx = ExecCtx::new(run_id, DummyServices);

        executor
            .execute(&mut ctx, make_state_with_input(run_id, "branch me"))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn validator_failure_emits_event_and_fails_run() {
        let mut workflow = CompiledWorkflow::new("validation", "done");
        workflow.add_node(GraphNode::validator(
            "done",
            BoxedValidator::new(FailingValidator),
        ));

        let executor = Executor::new(workflow).unwrap();
        let trace = RecordingTraceSink::new();
        let run_id = naaf_schema::state::RunId::new();
        let mut ctx = ExecCtx::new(run_id, DummyServices).with_trace(Box::new(trace.clone()));

        let error = executor
            .execute(&mut ctx, make_state(run_id))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("not valid"));

        let events = trace.events();
        assert!(
            events
                .iter()
                .any(|event| matches!(event, ExecutionEvent::ValidatorFailed { .. }))
        );
        assert!(
            events
                .iter()
                .any(|event| matches!(event, ExecutionEvent::RunFailed { .. }))
        );
    }
}
