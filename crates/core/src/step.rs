use std::{
    any::type_name,
    borrow::Cow,
    fmt::{Debug, Display, Formatter},
    marker::PhantomData,
    sync::Arc,
};

use futures::future::{LocalBoxFuture, try_join};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use tracing::{Instrument, debug, error, info, info_span, trace, warn};

use crate::{
    check::Check,
    checkpoint::{AttemptCheckpoint, StepCheckpoint, StepCheckpointer},
    materialiser::Materialiser,
    repair::{Attempt, AttemptReport, RepairPlanner, RetryPolicy, StepReport, Traced},
    span::{action, component, name, reason},
    task::Task,
};

type Runner<R, I, O, F, E> = dyn for<'a> Fn(&'a R, StepInput<I>) -> LocalBoxFuture<'a, Result<Traced<O, F>, StepError<F, E>>>
    + 'static;
type PipelineRunner<R, A, S, F, E> =
    dyn for<'a> Fn(&'a R, A) -> LocalBoxFuture<'a, Result<(S, Vec<F>), E>> + 'static;

enum StepInput<I> {
    Fresh {
        input: I,
    },
    Resume {
        input: I,
        checkpoint: StepCheckpoint,
    },
}
type BuilderMarker<R, I, A, S, F, E, State> = PhantomData<fn() -> (R, I, A, S, F, E, State)>;
type BuilderFor<T> = OpenStepBuilder<
    <T as Task>::Runtime,
    <T as Task>::Input,
    <T as Task>::Output,
    <T as Task>::Output,
    <T as Task>::Error,
>;

/// A composable workflow node with a local task-check-repair loop.
pub struct Step<R, I, O, F, E> {
    runner: Arc<Runner<R, I, O, F, E>>,
}

impl<R, I, O, F, E> Clone for Step<R, I, O, F, E> {
    fn clone(&self) -> Self {
        Self {
            runner: self.runner.clone(),
        }
    }
}

/// Marker state used before a builder has bound its public finding type.
pub struct FindingOpen;

/// Marker state used after a builder has bound its public finding type.
pub struct FindingBound;

/// Builder alias used before the step's public finding type has been selected.
pub type OpenStepBuilder<R, I, A, S, E> = StepBuilder<R, I, A, S, (), E, FindingOpen>;

/// Builder alias used after the step's public finding type has been selected.
pub type BoundStepBuilder<R, I, A, S, F, E> = StepBuilder<R, I, A, S, F, E, FindingBound>;

impl Step<(), (), (), (), ()> {
    /// Starts building a step around the given task.
    pub fn builder<T>(task: T) -> BuilderFor<T>
    where
        T: Task + 'static,
        T::Runtime: 'static,
        T::Input: 'static,
        T::Output: 'static,
        T::Error: 'static,
    {
        StepBuilder {
            task_name: type_name::<T>(),
            task_label: task.label(),
            task: Arc::new(task),
            pipeline: ValidationPipeline::identity(),
            repair: None,
            retry_policy: RetryPolicy::default(),
            step_checkpointer: None,
            marker: PhantomData,
        }
    }

    /// Builds a step directly from a closure, with no validation or repair.
    pub fn task<R, I, O, E, F>(f: F) -> Step<R, I, O, (), E>
    where
        R: 'static,
        I: Clone + 'static,
        O: Clone + 'static,
        E: 'static,
        F: Fn(&R, I) -> LocalBoxFuture<'_, Result<O, E>> + 'static,
    {
        Step::builder(crate::adaptor::TaskFn::new(f)).build()
    }
}

impl<R, I, O, F, E> Step<R, I, O, F, E> {
    /// Runs the step and returns only the final output.
    pub fn run<'a>(
        &'a self,
        runtime: &'a R,
        input: I,
    ) -> LocalBoxFuture<'a, Result<O, StepError<F, E>>>
    where
        R: 'static,
        I: 'static,
        O: 'static,
        F: 'static,
        E: 'static,
    {
        Box::pin(async move {
            self.run_traced(runtime, input)
                .await
                .map(Traced::into_output)
        })
    }

    /// Runs the step and returns the final output plus attempt metadata.
    pub fn run_traced<'a>(
        &'a self,
        runtime: &'a R,
        input: I,
    ) -> LocalBoxFuture<'a, Result<Traced<O, F>, StepError<F, E>>>
    where
        R: 'static,
        I: 'static,
        O: 'static,
        F: 'static,
        E: 'static,
    {
        (self.runner)(runtime, StepInput::Fresh { input })
    }

    /// Resumes a step from a previously saved checkpoint, continuing the retry loop
    /// from where it left off.
    ///
    /// Earlier attempts recorded in the checkpoint are preserved in the report
    /// but are not re-executed. The current input is deserialised and used as
    /// the next task input.
    pub fn run_resumed<'a>(
        &'a self,
        runtime: &'a R,
        checkpoint: StepCheckpoint,
    ) -> LocalBoxFuture<'a, Result<Traced<O, F>, StepError<F, E>>>
    where
        R: 'static,
        I: DeserializeOwned + Clone + Serialize + 'static,
        O: 'static,
        F: DeserializeOwned + Clone + Serialize + 'static,
        E: 'static,
    {
        let input: I = match serde_json::from_value(checkpoint.current_input.clone()) {
            Ok(input) => input,
            Err(_) => {
                #[allow(unreachable_code)]
                return Box::pin(async move {
                    Err(StepError::System {
                        stage: SystemStage::Task,
                        error: todo!(),
                    })
                });
            }
        };
        (self.runner)(runtime, StepInput::Resume { input, checkpoint })
    }

    /// Sequences two steps so the left output becomes the right input.
    pub fn then<Next>(self, next: Step<R, O, Next, F, E>) -> Step<R, I, Next, F, E>
    where
        R: 'static,
        I: 'static,
        O: 'static,
        Next: 'static,
        F: 'static,
        E: 'static,
    {
        let left = self.runner.clone();
        let right = next.runner.clone();

        Step {
            runner: Arc::new(move |runtime, step_input| {
                let left = left.clone();
                let right = right.clone();
                Box::pin(async move {
                    let left_traced = left(runtime, step_input).await?;
                    let (output, left_report) = left_traced.into_parts();
                    let right_traced = right(runtime, StepInput::Fresh { input: output }).await?;
                    let (next_output, right_report) = right_traced.into_parts();
                    Ok(Traced::new(next_output, left_report.extend(right_report)))
                })
            }),
        }
    }

    /// Fan-in helper equivalent to [`Step::then`] after a join or zip.
    pub fn reconcile<Next>(self, step: Step<R, O, Next, F, E>) -> Step<R, I, Next, F, E>
    where
        R: 'static,
        I: 'static,
        O: 'static,
        Next: 'static,
        F: 'static,
        E: 'static,
    {
        self.then(step)
    }

    /// Wraps a task in a step and uses it as a reconciliation stage.
    pub fn reconcile_task<T, Next>(self, task: T) -> Step<R, I, Next, F, E>
    where
        R: 'static,
        I: 'static,
        O: Clone + 'static,
        Next: Clone + 'static,
        F: Clone + 'static,
        E: 'static,
        T: Task<Runtime = R, Input = O, Output = Next, Error = E> + 'static,
    {
        self.reconcile(Step::builder(task).with_findings::<F>().build())
    }

    /// Maps a successful output while preserving trace metadata.
    pub fn map<Next, Map>(self, map: Map) -> Step<R, I, Next, F, E>
    where
        R: 'static,
        I: 'static,
        O: 'static,
        Next: 'static,
        F: 'static,
        E: 'static,
        Map: Fn(O) -> Next + 'static,
    {
        let runner = self.runner.clone();
        let map = Arc::new(map);

        Step {
            runner: Arc::new(move |runtime, input| {
                let runner = runner.clone();
                let map = map.clone();
                Box::pin(async move {
                    let traced = runner(runtime, input).await?;
                    let (output, report) = traced.into_parts();
                    Ok(Traced::new(map(output), report))
                })
            }),
        }
    }

    /// Runs two steps in parallel against the same cloned input.
    pub fn join<Other>(self, other: Step<R, I, Other, F, E>) -> Step<R, I, (O, Other), F, E>
    where
        R: 'static,
        I: Clone + 'static,
        O: 'static,
        Other: 'static,
        F: 'static,
        E: 'static,
    {
        let left = self.runner.clone();
        let right = other.runner.clone();

        Step {
            runner: Arc::new(move |runtime, step_input: StepInput<I>| {
                let left = left.clone();
                let right = right.clone();
                Box::pin(async move {
                    let input = match step_input {
                        StepInput::Fresh { input } => input,
                        StepInput::Resume { input, .. } => input,
                    };
                    let left_fut = left(
                        runtime,
                        StepInput::Fresh {
                            input: input.clone(),
                        },
                    );
                    let right_fut = right(runtime, StepInput::Fresh { input });
                    let (left_traced, right_traced) = try_join(left_fut, right_fut).await?;
                    let (left_output, left_report) = left_traced.into_parts();
                    let (right_output, right_report) = right_traced.into_parts();
                    Ok(Traced::new(
                        (left_output, right_output),
                        left_report.extend(right_report),
                    ))
                })
            }),
        }
    }

    /// Runs two steps in parallel against separate inputs.
    pub fn zip<OtherInput, OtherOutput>(
        self,
        other: Step<R, OtherInput, OtherOutput, F, E>,
    ) -> Step<R, (I, OtherInput), (O, OtherOutput), F, E>
    where
        R: 'static,
        I: 'static,
        O: 'static,
        OtherInput: 'static,
        OtherOutput: 'static,
        F: 'static,
        E: 'static,
    {
        let left = self.runner.clone();
        let right = other.runner.clone();

        Step {
            runner: Arc::new(move |runtime, step_input: StepInput<(I, OtherInput)>| {
                let left = left.clone();
                let right = right.clone();
                Box::pin(async move {
                    let (left_input, right_input) = match step_input {
                        StepInput::Fresh { input } => input,
                        StepInput::Resume { input, .. } => input,
                    };
                    let left_fut = left(runtime, StepInput::Fresh { input: left_input });
                    let right_fut = right(runtime, StepInput::Fresh { input: right_input });
                    let (left_traced, right_traced) = try_join(left_fut, right_fut).await?;
                    let (left_output, left_report) = left_traced.into_parts();
                    let (right_output, right_report) = right_traced.into_parts();
                    Ok(Traced::new(
                        (left_output, right_output),
                        left_report.extend(right_report),
                    ))
                })
            }),
        }
    }
}

fn build_step_checkpoint<I, A, F>(
    initial_input: &Value,
    current_input: &I,
    repair_attempts: &[Attempt<I, A, F>],
    report_attempts: &[AttemptReport<F>],
    retry_policy: RetryPolicy,
) -> StepCheckpoint
where
    I: Serialize,
    A: Serialize,
    F: Serialize + Clone,
{
    let report_attempts: Vec<crate::repair::AttemptReport<Value>> = report_attempts
        .iter()
        .map(|ar| crate::repair::AttemptReport {
            findings: ar
                .findings
                .iter()
                .cloned()
                .map(|f| serde_json::to_value(f).unwrap_or(Value::Null))
                .collect(),
            accepted: ar.accepted,
        })
        .collect();

    let repair_attempts: Vec<AttemptCheckpoint> = repair_attempts
        .iter()
        .map(|a| AttemptCheckpoint {
            input: serde_json::to_value(&a.input).unwrap_or(Value::Null),
            artefact: serde_json::to_value(&a.artefact).unwrap_or(Value::Null),
            findings: a
                .findings
                .iter()
                .cloned()
                .map(|f| serde_json::to_value(f).unwrap_or(Value::Null))
                .collect(),
        })
        .collect();

    StepCheckpoint {
        initial_input: initial_input.clone(),
        current_input: serde_json::to_value(current_input).unwrap_or(Value::Null),
        repair_attempts,
        report_attempts,
        retry_policy,
    }
}

/// Builder for configuring a step's checks, materialisation, and repair loop.
pub struct StepBuilder<R, I, A, S, F, E, State = FindingBound> {
    task_name: &'static str,
    task_label: Option<Cow<'static, str>>,
    task: Arc<dyn Task<Runtime = R, Input = I, Output = A, Error = E>>,
    pipeline: ValidationPipeline<R, A, S, F, E>,
    repair: Option<
        Arc<dyn RepairPlanner<Runtime = R, Input = I, Artefact = A, Finding = F, Error = E>>,
    >,
    retry_policy: RetryPolicy,
    step_checkpointer: Option<Arc<dyn StepCheckpointer>>,
    marker: BuilderMarker<R, I, A, S, F, E, State>,
}

impl<R, I, A, S, E> OpenStepBuilder<R, I, A, S, E>
where
    R: 'static,
    I: 'static,
    A: 'static,
    S: 'static,
    E: 'static,
{
    /// Binds the step to a finding type without adding a check yet.
    pub fn with_findings<F>(self) -> BoundStepBuilder<R, I, A, S, F, E>
    where
        F: 'static,
    {
        StepBuilder {
            task_name: self.task_name,
            task_label: self.task_label,
            task: self.task,
            pipeline: self.pipeline.bind_findings(),
            repair: None,
            retry_policy: self.retry_policy,
            step_checkpointer: self.step_checkpointer,
            marker: PhantomData,
        }
    }

    /// Adds the first check and binds the builder's finding type to that check.
    pub fn validate<C>(self, check: C) -> BoundStepBuilder<R, I, A, S, C::Finding, E>
    where
        S: Clone + 'static,
        C: Check<Runtime = R, Subject = S, Error = E> + 'static,
        C::Finding: 'static,
    {
        StepBuilder {
            task_name: self.task_name,
            task_label: self.task_label,
            task: self.task,
            pipeline: self.pipeline.validate_first(check),
            repair: None,
            retry_policy: self.retry_policy,
            step_checkpointer: self.step_checkpointer,
            marker: PhantomData,
        }
    }
}

impl<R, I, A, S, F, E, State> StepBuilder<R, I, A, S, F, E, State>
where
    R: 'static,
    I: 'static,
    A: 'static,
    S: 'static,
    F: 'static,
    E: 'static,
{
    /// Materialises the current validation subject into a new subject type.
    pub fn materialise<M, NextSubject>(
        self,
        materialiser: M,
    ) -> StepBuilder<R, I, A, NextSubject, F, E, State>
    where
        M: Materialiser<Runtime = R, Input = S, Output = NextSubject, Error = E> + 'static,
        NextSubject: 'static,
    {
        StepBuilder {
            task_name: self.task_name,
            task_label: self.task_label,
            task: self.task,
            pipeline: self.pipeline.materialise(materialiser),
            repair: self.repair,
            retry_policy: self.retry_policy,
            step_checkpointer: self.step_checkpointer,
            marker: PhantomData,
        }
    }

    /// Sets the retry policy used by the built step.
    pub fn retry_policy(mut self, retry_policy: RetryPolicy) -> Self {
        self.retry_policy = retry_policy;
        self
    }

    /// Overrides the human-facing label emitted for this step.
    pub fn with_label(mut self, label: impl Into<Cow<'static, str>>) -> Self {
        self.task_label = Some(label.into());
        self
    }

    /// Installs a step checkpointer for saving retry loop state after each attempt.
    pub fn checkpoint_with(mut self, checkpointer: impl StepCheckpointer + 'static) -> Self {
        self.step_checkpointer = Some(Arc::new(checkpointer));
        self
    }

    /// Finishes the builder and produces a runnable step without checkpointing.
    pub fn build(self) -> Step<R, I, A, F, E>
    where
        I: Clone + 'static,
        A: Clone + 'static,
        F: Clone + 'static,
    {
        let task = self.task;
        let task_name = self.task_name;
        let task_label = self.task_label;
        let pipeline = self.pipeline;
        let repair = self.repair;
        let retry_policy = self.retry_policy;

        Step {
            runner: Arc::new(move |runtime, step_input: StepInput<I>| {
                let task = task.clone();
                let pipeline = pipeline.clone();
                let repair = repair.clone();
                let step_span = if let Some(task_label) = task_label.clone() {
                    info_span!(
                        name::STEP,
                        component = component::STEP,
                        task = task_name,
                        label = task_label.as_ref(),
                        input_type = %type_name::<I>(),
                        artefact_type = %type_name::<A>(),
                        finding_type = %type_name::<F>(),
                        max_attempts = retry_policy.max_attempts()
                    )
                } else {
                    info_span!(
                        name::STEP,
                        component = component::STEP,
                        task = task_name,
                        input_type = %type_name::<I>(),
                        artefact_type = %type_name::<A>(),
                        finding_type = %type_name::<F>(),
                        max_attempts = retry_policy.max_attempts()
                    )
                };

                Box::pin(
                    async move {
                        let mut input = match step_input {
                            StepInput::Fresh { input } => input,
                            StepInput::Resume { .. } => {
                                #[allow(unreachable_code)]
                                return Err(StepError::System {
                                    stage: SystemStage::Task,
                                    error: todo!(),
                                });
                            }
                        };
                        let mut repair_attempts: Vec<Attempt<I, A, F>> = Vec::new();
                        let mut report_attempts: Vec<AttemptReport<F>> = Vec::new();

                        info!(action = action::RUN_START, "step started");
                        trace!(action = action::INPUT, "step input received");

                        loop {
                            let attempt = repair_attempts.len() + 1;
                            debug!(
                                action = action::ATTEMPT_START,
                                attempt, "step attempt started"
                            );

                            let artefact =
                                task.run(runtime, input.clone()).await.map_err(|error| {
                                    error!(
                                        action = action::RUN_ERROR,
                                        attempt,
                                        stage = %SystemStage::Task,
                                        "step failed with system error"
                                    );
                                    StepError::System {
                                        stage: SystemStage::Task,
                                        error,
                                    }
                                })?;

                            trace!(
                                action = action::ATTEMPT_OUTPUT,
                                attempt, "task produced artefact"
                            );

                            let (_, findings): (_, Vec<F>) = pipeline
                                .run(runtime, artefact.clone())
                                .await
                                .map_err(|error| {
                                    error!(
                                        action = action::RUN_ERROR,
                                        attempt,
                                        stage = %SystemStage::Validation,
                                        "step failed with system error"
                                    );
                                    StepError::System {
                                        stage: SystemStage::Validation,
                                        error,
                                    }
                                })?;

                            let accepted = findings.is_empty();
                            let finding_count = findings.len();
                            report_attempts.push(AttemptReport {
                                findings: findings.clone(),
                                accepted,
                            });

                            debug!(
                                action = action::ATTEMPT_VALIDATED,
                                attempt, accepted, finding_count, "step attempt validated"
                            );

                            if accepted {
                                info!(
                                    action = action::RUN_COMPLETE,
                                    attempts = report_attempts.len(),
                                    "step completed"
                                );
                                return Ok(Traced::new(artefact, StepReport::new(report_attempts)));
                            }

                            repair_attempts.push(Attempt {
                                input: input.clone(),
                                artefact: artefact.clone(),
                                findings,
                            });

                            if repair_attempts.len() >= retry_policy.max_attempts() {
                                warn!(
                                    action = action::RUN_REJECTED,
                                    attempts = report_attempts.len(),
                                    reason = reason::RETRY_LIMIT_REACHED,
                                    "step rejected"
                                );
                                return Err(StepError::Rejected(StepReport::new(report_attempts)));
                            }

                            let Some(repair) = repair.clone() else {
                                warn!(
                                    action = action::RUN_REJECTED,
                                    attempts = report_attempts.len(),
                                    reason = reason::REPAIR_UNAVAILABLE,
                                    "step rejected"
                                );
                                return Err(StepError::Rejected(StepReport::new(report_attempts)));
                            };

                            debug!(
                                action = action::ATTEMPT_REPAIR_START,
                                attempt,
                                next_attempt = attempt + 1,
                                "planning repair attempt"
                            );

                            input = repair
                                .repair(runtime, repair_attempts.clone())
                                .await
                                .map_err(|error| {
                                    error!(
                                        action = action::RUN_ERROR,
                                        attempt,
                                        stage = %SystemStage::Repair,
                                        "step failed with system error"
                                    );
                                    StepError::System {
                                        stage: SystemStage::Repair,
                                        error,
                                    }
                                })?;

                            trace!(
                                action = action::ATTEMPT_REPAIR_COMPLETE,
                                attempt,
                                next_attempt = attempt + 1,
                                "repair planner produced next input"
                            );
                        }
                    }
                    .instrument(step_span),
                )
            }),
        }
    }

    /// Finishes the builder and produces a step that supports checkpointing and
    /// resume. The input, artefact, and finding types must be serialisable and
    /// deserialisable so their values can be persisted between sessions.
    pub fn build_persistent(self) -> Step<R, I, A, F, E>
    where
        I: Clone + Serialize + DeserializeOwned + 'static,
        A: Clone + Serialize + DeserializeOwned + 'static,
        F: Clone + Serialize + DeserializeOwned + 'static,
    {
        let task = self.task;
        let task_name = self.task_name;
        let task_label = self.task_label;
        let pipeline = self.pipeline;
        let repair = self.repair;
        let retry_policy = self.retry_policy;
        let step_checkpointer = self.step_checkpointer;

        Step {
            runner: Arc::new(move |runtime, step_input: StepInput<I>| {
                let task = task.clone();
                let pipeline = pipeline.clone();
                let repair = repair.clone();
                let step_checkpointer = step_checkpointer.clone();
                let step_span = if let Some(task_label) = task_label.clone() {
                    info_span!(
                        name::STEP,
                        component = component::STEP,
                        task = task_name,
                        label = task_label.as_ref(),
                        input_type = %type_name::<I>(),
                        artefact_type = %type_name::<A>(),
                        finding_type = %type_name::<F>(),
                        max_attempts = retry_policy.max_attempts()
                    )
                } else {
                    info_span!(
                        name::STEP,
                        component = component::STEP,
                        task = task_name,
                        input_type = %type_name::<I>(),
                        artefact_type = %type_name::<A>(),
                        finding_type = %type_name::<F>(),
                        max_attempts = retry_policy.max_attempts()
                    )
                };

                Box::pin(
                    async move {
                        let (initial_input, mut input, mut repair_attempts, mut report_attempts) =
                            match step_input {
                                StepInput::Fresh { input: fresh } => {
                                    let initial_input_value =
                                        serde_json::to_value(&fresh).unwrap_or(Value::Null);
                                    (initial_input_value, fresh, Vec::new(), Vec::new())
                                }
                                StepInput::Resume {
                                    input: resumed,
                                    checkpoint,
                                } => {
                                    let repair_attempts = checkpoint
                                        .repair_attempts
                                        .into_iter()
                                        .map(|ac| {
                                            let input: I = serde_json::from_value(ac.input)
                                                .unwrap_or_else(|_| todo!());
                                            let artefact: A = serde_json::from_value(ac.artefact)
                                                .unwrap_or_else(|_| todo!());
                                            let findings: Vec<F> = ac
                                                .findings
                                                .into_iter()
                                                .map(|v| {
                                                    serde_json::from_value(v)
                                                        .unwrap_or_else(|_| todo!())
                                                })
                                                .collect();
                                            Attempt {
                                                input,
                                                artefact,
                                                findings,
                                            }
                                        })
                                        .collect();
                                    let report_attempts = checkpoint
                                        .report_attempts
                                        .into_iter()
                                        .map(|ar| AttemptReport {
                                            findings: ar
                                                .findings
                                                .into_iter()
                                                .map(|v| {
                                                    serde_json::from_value(v)
                                                        .unwrap_or_else(|_| todo!())
                                                })
                                                .collect(),
                                            accepted: ar.accepted,
                                        })
                                        .collect();
                                    (
                                        checkpoint.initial_input,
                                        resumed,
                                        repair_attempts,
                                        report_attempts,
                                    )
                                }
                            };

                        info!(action = action::RUN_START, "step started");
                        trace!(action = action::INPUT, "step input received");

                        loop {
                            let attempt = repair_attempts.len() + 1;
                            debug!(
                                action = action::ATTEMPT_START,
                                attempt, "step attempt started"
                            );

                            let artefact =
                                task.run(runtime, input.clone()).await.map_err(|error| {
                                    error!(
                                        action = action::RUN_ERROR,
                                        attempt,
                                        stage = %SystemStage::Task,
                                        "step failed with system error"
                                    );
                                    StepError::System {
                                        stage: SystemStage::Task,
                                        error,
                                    }
                                })?;

                            trace!(
                                action = action::ATTEMPT_OUTPUT,
                                attempt, "task produced artefact"
                            );

                            let (_, findings): (_, Vec<F>) = pipeline
                                .run(runtime, artefact.clone())
                                .await
                                .map_err(|error| {
                                    error!(
                                        action = action::RUN_ERROR,
                                        attempt,
                                        stage = %SystemStage::Validation,
                                        "step failed with system error"
                                    );
                                    StepError::System {
                                        stage: SystemStage::Validation,
                                        error,
                                    }
                                })?;

                            let accepted = findings.is_empty();
                            let finding_count = findings.len();
                            report_attempts.push(AttemptReport {
                                findings: findings.clone(),
                                accepted,
                            });

                            debug!(
                                action = action::ATTEMPT_VALIDATED,
                                attempt, accepted, finding_count, "step attempt validated"
                            );

                            if let Some(cp) = &step_checkpointer {
                                let checkpoint = build_step_checkpoint(
                                    &initial_input,
                                    &input,
                                    &repair_attempts,
                                    &report_attempts,
                                    retry_policy,
                                );
                                cp.checkpoint(checkpoint).await;
                            }

                            if accepted {
                                info!(
                                    action = action::RUN_COMPLETE,
                                    attempts = report_attempts.len(),
                                    "step completed"
                                );
                                return Ok(Traced::new(artefact, StepReport::new(report_attempts)));
                            }

                            repair_attempts.push(Attempt {
                                input: input.clone(),
                                artefact: artefact.clone(),
                                findings,
                            });

                            if repair_attempts.len() >= retry_policy.max_attempts() {
                                warn!(
                                    action = action::RUN_REJECTED,
                                    attempts = report_attempts.len(),
                                    reason = reason::RETRY_LIMIT_REACHED,
                                    "step rejected"
                                );
                                return Err(StepError::Rejected(StepReport::new(report_attempts)));
                            }

                            let Some(repair) = repair.clone() else {
                                warn!(
                                    action = action::RUN_REJECTED,
                                    attempts = report_attempts.len(),
                                    reason = reason::REPAIR_UNAVAILABLE,
                                    "step rejected"
                                );
                                return Err(StepError::Rejected(StepReport::new(report_attempts)));
                            };

                            debug!(
                                action = action::ATTEMPT_REPAIR_START,
                                attempt,
                                next_attempt = attempt + 1,
                                "planning repair attempt"
                            );

                            input = repair
                                .repair(runtime, repair_attempts.clone())
                                .await
                                .map_err(|error| {
                                    error!(
                                        action = action::RUN_ERROR,
                                        attempt,
                                        stage = %SystemStage::Repair,
                                        "step failed with system error"
                                    );
                                    StepError::System {
                                        stage: SystemStage::Repair,
                                        error,
                                    }
                                })?;

                            trace!(
                                action = action::ATTEMPT_REPAIR_COMPLETE,
                                attempt,
                                next_attempt = attempt + 1,
                                "repair planner produced next input"
                            );
                        }
                    }
                    .instrument(step_span),
                )
            }),
        }
    }
}

impl<R, I, A, S, F, E> BoundStepBuilder<R, I, A, S, F, E>
where
    R: 'static,
    I: 'static,
    A: 'static,
    S: 'static,
    F: 'static,
    E: 'static,
{
    /// Adds a check whose findings already match the builder's finding type.
    pub fn validate<C>(self, check: C) -> Self
    where
        S: Clone + 'static,
        C: Check<Runtime = R, Subject = S, Finding = F, Error = E> + 'static,
    {
        Self {
            task_name: self.task_name,
            task_label: self.task_label,
            task: self.task,
            pipeline: self.pipeline.validate(check),
            repair: self.repair,
            retry_policy: self.retry_policy,
            step_checkpointer: self.step_checkpointer,
            marker: PhantomData,
        }
    }

    /// Adds a check whose findings can be converted into the builder's finding type.
    pub fn validate_into<C>(self, check: C) -> Self
    where
        S: Clone + 'static,
        C: Check<Runtime = R, Subject = S, Error = E> + 'static,
        C::Finding: Into<F> + 'static,
    {
        Self {
            task_name: self.task_name,
            task_label: self.task_label,
            task: self.task,
            pipeline: self.pipeline.validate_into(check),
            repair: self.repair,
            retry_policy: self.retry_policy,
            step_checkpointer: self.step_checkpointer,
            marker: PhantomData,
        }
    }

    /// Installs the planner used to generate retry inputs after failed attempts.
    pub fn repair_with<P>(self, planner: P) -> Self
    where
        P: RepairPlanner<Runtime = R, Input = I, Artefact = A, Finding = F, Error = E> + 'static,
    {
        Self {
            task_name: self.task_name,
            task_label: self.task_label,
            task: self.task,
            pipeline: self.pipeline,
            repair: Some(Arc::new(planner)),
            retry_policy: self.retry_policy,
            step_checkpointer: self.step_checkpointer,
            marker: PhantomData,
        }
    }
}

/// Identifies the subsystem that produced a system-level step error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemStage {
    /// The task itself failed to execute.
    Task,
    /// A check or materialiser failed to execute.
    Validation,
    /// The repair planner failed to produce a new input.
    Repair,
}

/// Errors returned while running a step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StepError<F, E> {
    /// An unrecoverable infrastructure error occurred.
    System { stage: SystemStage, error: E },
    /// The step exhausted retries or had no repair planner to continue.
    Rejected(StepReport<F>),
}

impl<F, E> Display for StepError<F, E>
where
    E: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::System { stage, error } => write!(f, "{stage} failed: {error}"),
            Self::Rejected(report) => write!(
                f,
                "step rejected after {} attempt(s)",
                report.attempt_count()
            ),
        }
    }
}

impl Display for SystemStage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Task => f.write_str("task execution"),
            Self::Validation => f.write_str("validation"),
            Self::Repair => f.write_str("repair planning"),
        }
    }
}

impl<F, E> std::error::Error for StepError<F, E>
where
    F: Debug,
    E: std::error::Error + 'static,
{
}

struct ValidationPipeline<R, A, S, F, E> {
    run: Arc<PipelineRunner<R, A, S, F, E>>,
}

impl<R, A, S, F, E> Clone for ValidationPipeline<R, A, S, F, E> {
    fn clone(&self) -> Self {
        Self {
            run: self.run.clone(),
        }
    }
}

impl<R, A, F, E> ValidationPipeline<R, A, A, F, E>
where
    R: 'static,
    A: 'static,
    F: 'static,
    E: 'static,
{
    fn identity() -> Self {
        Self {
            run: Arc::new(|_, artefact| Box::pin(async move { Ok((artefact, Vec::new())) })),
        }
    }
}

impl<R, A, S, F, E> ValidationPipeline<R, A, S, F, E>
where
    R: 'static,
    A: 'static,
    S: 'static,
    F: 'static,
    E: 'static,
{
    fn run<'a>(
        &'a self,
        runtime: &'a R,
        artefact: A,
    ) -> LocalBoxFuture<'a, Result<(S, Vec<F>), E>> {
        (self.run)(runtime, artefact)
    }

    fn bind_findings<NextFinding>(self) -> ValidationPipeline<R, A, S, NextFinding, E>
    where
        NextFinding: 'static,
    {
        let run = self.run.clone();

        ValidationPipeline {
            run: Arc::new(move |runtime, artefact| {
                let run = run.clone();
                Box::pin(async move {
                    let (subject, _) = run(runtime, artefact).await?;
                    Ok((subject, Vec::new()))
                })
            }),
        }
    }

    fn validate_first<C>(self, check: C) -> ValidationPipeline<R, A, S, C::Finding, E>
    where
        S: Clone + 'static,
        C: Check<Runtime = R, Subject = S, Error = E> + 'static,
        C::Finding: 'static,
    {
        let run = self.run.clone();
        let check = Arc::new(check);

        ValidationPipeline {
            run: Arc::new(move |runtime, artefact| {
                let run = run.clone();
                let check = check.clone();
                Box::pin(async move {
                    let (subject, _) = run(runtime, artefact).await?;
                    let findings = check.check(runtime, subject.clone()).await?;
                    Ok((subject, findings))
                })
            }),
        }
    }

    fn validate<C>(self, check: C) -> Self
    where
        S: Clone + 'static,
        C: Check<Runtime = R, Subject = S, Finding = F, Error = E> + 'static,
    {
        let run = self.run.clone();
        let check = Arc::new(check);

        Self {
            run: Arc::new(move |runtime, artefact| {
                let run = run.clone();
                let check = check.clone();
                Box::pin(async move {
                    let (subject, mut findings) = run(runtime, artefact).await?;
                    findings.extend(check.check(runtime, subject.clone()).await?);
                    Ok((subject, findings))
                })
            }),
        }
    }

    fn validate_into<C>(self, check: C) -> Self
    where
        S: Clone + 'static,
        C: Check<Runtime = R, Subject = S, Error = E> + 'static,
        C::Finding: Into<F> + 'static,
    {
        let run = self.run.clone();
        let check = Arc::new(check);

        Self {
            run: Arc::new(move |runtime, artefact| {
                let run = run.clone();
                let check = check.clone();
                Box::pin(async move {
                    let (subject, mut findings) = run(runtime, artefact).await?;
                    findings.extend(
                        check
                            .check(runtime, subject.clone())
                            .await?
                            .into_iter()
                            .map(Into::into),
                    );
                    Ok((subject, findings))
                })
            }),
        }
    }

    fn materialise<M, NextSubject>(
        self,
        materialiser: M,
    ) -> ValidationPipeline<R, A, NextSubject, F, E>
    where
        M: Materialiser<Runtime = R, Input = S, Output = NextSubject, Error = E> + 'static,
        NextSubject: 'static,
    {
        let run = self.run.clone();
        let materialiser = Arc::new(materialiser);

        ValidationPipeline {
            run: Arc::new(move |runtime, artefact| {
                let run = run.clone();
                let materialiser = materialiser.clone();
                Box::pin(async move {
                    let (subject, findings) = run(runtime, artefact).await?;
                    let next_subject = materialiser.materialise(runtime, subject).await?;
                    Ok((next_subject, findings))
                })
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Formatter;

    use futures::future::LocalBoxFuture;

    use super::Step;
    use crate::{
        check::Check,
        materialiser::Materialiser,
        repair::{Attempt, RepairPlanner, RetryPolicy},
        task::Task,
    };

    #[derive(Debug)]
    struct TestRuntime {
        required_revision: usize,
        repair_increment: usize,
        require_even_revision: bool,
        increment: usize,
        multiplier: usize,
        reconcile_bias: usize,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct CodeInput {
        prompt: &'static str,
        revision: usize,
        failing_test: Option<&'static str>,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct Patch {
        revision: usize,
        passes_tests: bool,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum Finding {
        TestFailure(&'static str),
        OddRevision,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct TestWorktree {
        patch: Patch,
        required_revision: usize,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct TestError;

    impl std::fmt::Display for TestError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            f.write_str("test error")
        }
    }

    impl std::error::Error for TestError {}

    struct Generator;

    impl Task for Generator {
        type Runtime = TestRuntime;
        type Input = CodeInput;
        type Output = Patch;
        type Error = TestError;

        fn run<'a>(
            &'a self,
            runtime: &'a Self::Runtime,
            input: Self::Input,
        ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
            Box::pin(async move {
                Ok(Patch {
                    revision: input.revision,
                    passes_tests: input.revision >= runtime.required_revision,
                })
            })
        }
    }

    struct CargoTest;

    impl Materialiser for CargoTest {
        type Runtime = TestRuntime;
        type Input = Patch;
        type Output = TestWorktree;
        type Error = TestError;

        fn materialise<'a>(
            &'a self,
            runtime: &'a Self::Runtime,
            input: Self::Input,
        ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
            Box::pin(async move {
                Ok(TestWorktree {
                    patch: input,
                    required_revision: runtime.required_revision,
                })
            })
        }
    }

    struct PatchShapeCheck;

    impl Check for PatchShapeCheck {
        type Runtime = TestRuntime;
        type Subject = Patch;
        type Finding = Finding;
        type Error = TestError;

        fn check<'a>(
            &'a self,
            runtime: &'a Self::Runtime,
            subject: Self::Subject,
        ) -> LocalBoxFuture<'a, Result<Vec<Self::Finding>, Self::Error>> {
            Box::pin(async move {
                if runtime.require_even_revision && subject.revision % 2 != 0 {
                    Ok(vec![Finding::OddRevision])
                } else {
                    Ok(Vec::new())
                }
            })
        }
    }

    struct TestCheck;

    impl Check for TestCheck {
        type Runtime = TestRuntime;
        type Subject = TestWorktree;
        type Finding = Finding;
        type Error = TestError;

        fn check<'a>(
            &'a self,
            _runtime: &'a Self::Runtime,
            subject: Self::Subject,
        ) -> LocalBoxFuture<'a, Result<Vec<Self::Finding>, Self::Error>> {
            Box::pin(async move {
                if subject.patch.passes_tests && subject.patch.revision >= subject.required_revision
                {
                    Ok(Vec::new())
                } else {
                    Ok(vec![Finding::TestFailure("cargo test")])
                }
            })
        }
    }

    struct Repair;

    impl RepairPlanner for Repair {
        type Runtime = TestRuntime;
        type Input = CodeInput;
        type Artefact = Patch;
        type Finding = Finding;
        type Error = TestError;

        fn repair<'a>(
            &'a self,
            runtime: &'a Self::Runtime,
            attempts: Vec<Attempt<Self::Input, Self::Artefact, Self::Finding>>,
        ) -> LocalBoxFuture<'a, Result<Self::Input, Self::Error>> {
            Box::pin(async move {
                let previous = attempts.last().expect("attempt present");
                Ok(CodeInput {
                    prompt: previous.input.prompt,
                    revision: previous.artefact.revision + runtime.repair_increment,
                    failing_test: Some("cargo test"),
                })
            })
        }
    }

    struct Increment;

    impl Task for Increment {
        type Runtime = TestRuntime;
        type Input = usize;
        type Output = usize;
        type Error = TestError;

        fn run<'a>(
            &'a self,
            runtime: &'a Self::Runtime,
            input: Self::Input,
        ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
            Box::pin(async move { Ok(input + runtime.increment) })
        }
    }

    struct Double;

    impl Task for Double {
        type Runtime = TestRuntime;
        type Input = usize;
        type Output = usize;
        type Error = TestError;

        fn run<'a>(
            &'a self,
            runtime: &'a Self::Runtime,
            input: Self::Input,
        ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
            Box::pin(async move { Ok(input * runtime.multiplier) })
        }
    }

    struct SumPair;

    impl Task for SumPair {
        type Runtime = TestRuntime;
        type Input = (usize, usize);
        type Output = usize;
        type Error = TestError;

        fn run<'a>(
            &'a self,
            runtime: &'a Self::Runtime,
            input: Self::Input,
        ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
            Box::pin(async move { Ok(input.0 + input.1 + runtime.reconcile_bias) })
        }
    }

    fn runtime() -> TestRuntime {
        TestRuntime {
            required_revision: 2,
            repair_increment: 1,
            require_even_revision: true,
            increment: 1,
            multiplier: 2,
            reconcile_bias: 3,
        }
    }

    #[tokio::test]
    async fn step_retries_until_checks_pass() {
        let step = Step::builder(Generator)
            .validate(PatchShapeCheck)
            .materialise(CargoTest)
            .validate(TestCheck)
            .repair_with(Repair)
            .retry_policy(RetryPolicy::new(3))
            .build();

        let result = step
            .run(
                &runtime(),
                CodeInput {
                    prompt: "add feature",
                    revision: 0,
                    failing_test: None,
                },
            )
            .await
            .expect("step should recover");

        assert_eq!(
            result,
            Patch {
                revision: 2,
                passes_tests: true,
            }
        );
    }

    #[tokio::test]
    async fn step_returns_trace_on_success() {
        let step = Step::builder(Generator)
            .validate(PatchShapeCheck)
            .materialise(CargoTest)
            .validate(TestCheck)
            .repair_with(Repair)
            .retry_policy(RetryPolicy::new(3))
            .build();

        let traced = step
            .run_traced(
                &runtime(),
                CodeInput {
                    prompt: "add feature",
                    revision: 0,
                    failing_test: None,
                },
            )
            .await
            .expect("step should recover");

        assert_eq!(traced.output().revision, 2);
        assert_eq!(traced.report().attempt_count(), 3);
        assert!(!traced.report().attempts()[0].accepted());
        assert!(!traced.report().attempts()[1].accepted());
        assert!(traced.report().attempts()[2].accepted());
        assert!(traced.report().attempts()[2].findings.is_empty());
    }

    #[tokio::test]
    async fn step_returns_report_when_retries_exhausted() {
        let step = Step::builder(Generator)
            .materialise(CargoTest)
            .validate(TestCheck)
            .retry_policy(RetryPolicy::new(1))
            .build();

        let error = step
            .run(
                &runtime(),
                CodeInput {
                    prompt: "add feature",
                    revision: 0,
                    failing_test: None,
                },
            )
            .await
            .expect_err("step should reject");

        match error {
            super::StepError::Rejected(report) => {
                assert_eq!(report.attempt_count(), 1);
                assert!(!report.attempts()[0].accepted());
                assert_eq!(
                    report.attempts()[0].findings,
                    vec![Finding::TestFailure("cargo test")]
                );
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[tokio::test]
    async fn steps_compose_with_then_join_zip_and_reconcile() {
        let add_one = Step::builder(Increment).with_findings::<Finding>().build();
        let double = Step::builder(Double).with_findings::<Finding>().build();

        let sequenced = add_one.clone().then(double.clone());
        let joined = add_one.clone().join(double.clone());
        let reconciled = joined.clone().reconcile_task(SumPair);
        let zipped = add_one.zip(double);
        let test_runtime = runtime();

        assert_eq!(
            sequenced
                .run(&test_runtime, 3)
                .await
                .expect("sequence result"),
            8
        );
        assert_eq!(
            joined.run(&test_runtime, 3).await.expect("join result"),
            (4, 6)
        );
        assert_eq!(
            reconciled
                .run(&test_runtime, 3)
                .await
                .expect("reconcile result"),
            13
        );
        assert_eq!(
            zipped.run(&test_runtime, (3, 4)).await.expect("zip result"),
            (4, 8)
        );
    }
}
