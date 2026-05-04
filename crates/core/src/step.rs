use std::{
    any::type_name,
    borrow::Cow,
    fmt::{Debug, Display, Formatter},
    marker::PhantomData,
    sync::Arc,
};

use futures::future::LocalBoxFuture;
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

/// Builder alias used after the step's public finding type has been selected.
pub type BoundStepBuilder<R, I, A, S, F, E> = StepBuilder<R, I, A, S, F, E, FindingBound>;
type BuilderFor<T> = OpenStepBuilder<
    <T as Task>::Runtime,
    <T as Task>::Input,
    <T as Task>::Output,
    <T as Task>::Output,
    <T as Task>::Error,
>;
type BuilderMarker<R, I, A, S, F, E, State> = PhantomData<fn() -> (R, I, A, S, F, E, State)>;
/// Builder alias used before the step's public finding type has been selected.
pub type OpenStepBuilder<R, I, A, S, E> = StepBuilder<R, I, A, S, (), E, FindingOpen>;
type PipelineRunner<R, I, A, S, F, E> =
    dyn for<'a> Fn(&'a R, I, A) -> LocalBoxFuture<'a, Result<(S, Vec<F>), E>> + 'static;
type Runner<R, I, O, F, E> = dyn for<'a> Fn(&'a R, StepInput<I>) -> LocalBoxFuture<'a, Result<Traced<O, F>, StepError<F, E>>>
    + 'static;

/// A composable workflow node with a local task-check-repair loop.
pub struct Step<R, I, O, F, E> {
    runner: Arc<Runner<R, I, O, F, E>>,
}

enum StepInput<I> {
    Fresh {
        input: I,
    },
    Resume {
        input: I,
        checkpoint: StepCheckpoint,
    },
}

/// Marker state used before a builder has bound its public finding type.
pub struct FindingOpen;

/// Marker state used after a builder has bound its public finding type.
pub struct FindingBound;

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

struct ValidationPipeline<R, I, A, S, F, E> {
    run: Arc<PipelineRunner<R, I, A, S, F, E>>,
}

/// Builder for configuring a step's checks, materialisation, and repair loop.
pub struct StepBuilder<R, I, A, S, F, E, State = FindingBound> {
    task_name: &'static str,
    task_label: Option<Cow<'static, str>>,
    task: Arc<dyn Task<Runtime = R, Input = I, Output = A, Error = E>>,
    pipeline: ValidationPipeline<R, I, A, S, F, E>,
    repair:
        Option<Arc<dyn RepairPlanner<Runtime = R, Input = I, Output = A, Finding = F, Error = E>>>,
    retry_policy: RetryPolicy,
    step_checkpointer: Option<Arc<dyn StepCheckpointer>>,
    marker: BuilderMarker<R, I, A, S, F, E, State>,
}

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

    /// Maps an upstream input into the input expected by this step.
    ///
    /// This is useful when composing a step that needs only a projected or
    /// reshaped view of a larger workflow context.
    pub fn map_input<PreviousInput, Map>(self, map: Map) -> Step<R, PreviousInput, O, F, E>
    where
        R: 'static,
        PreviousInput: 'static,
        I: 'static,
        O: 'static,
        F: 'static,
        E: 'static,
        Map: Fn(PreviousInput) -> I + 'static,
    {
        let runner = self.runner.clone();
        let map = Arc::new(map);

        Step {
            runner: Arc::new(move |runtime, input| {
                let runner = runner.clone();
                let map = map.clone();
                Box::pin(async move {
                    let input = match input {
                        StepInput::Fresh { input } | StepInput::Resume { input, .. } => input,
                    };
                    runner(runtime, StepInput::Fresh { input: map(input) }).await
                })
            }),
        }
    }

    /// Alias for [`Step::map_input`] using the functional-programming term.
    pub fn contramap<PreviousInput, Map>(self, map: Map) -> Step<R, PreviousInput, O, F, E>
    where
        R: 'static,
        PreviousInput: 'static,
        I: 'static,
        O: 'static,
        F: 'static,
        E: 'static,
        Map: Fn(PreviousInput) -> I + 'static,
    {
        self.map_input(map)
    }

    /// Maps a successful output together with the input that produced it.
    pub fn map_with_input<Next, Map>(self, map: Map) -> Step<R, I, Next, F, E>
    where
        R: 'static,
        I: Clone + 'static,
        O: 'static,
        Next: 'static,
        F: 'static,
        E: 'static,
        Map: Fn(I, O) -> Next + 'static,
    {
        let runner = self.runner.clone();
        let map = Arc::new(map);

        Step {
            runner: Arc::new(move |runtime, input| {
                let runner = runner.clone();
                let map = map.clone();
                Box::pin(async move {
                    let original_input = match &input {
                        StepInput::Fresh { input } | StepInput::Resume { input, .. } => {
                            input.clone()
                        }
                    };
                    let traced = runner(runtime, input).await?;
                    let (output, report) = traced.into_parts();
                    Ok(Traced::new(map(original_input, output), report))
                })
            }),
        }
    }

    /// Returns each successful output paired with the input that produced it.
    pub fn with_input(self) -> Step<R, I, (I, O), F, E>
    where
        R: 'static,
        I: Clone + 'static,
        O: 'static,
        F: 'static,
        E: 'static,
    {
        self.map_with_input(|input, output| (input, output))
    }

    /// Maps findings in successful traces and rejection reports.
    pub fn map_findings<NextFinding, Map>(self, map: Map) -> Step<R, I, O, NextFinding, E>
    where
        R: 'static,
        I: 'static,
        O: 'static,
        F: 'static,
        NextFinding: 'static,
        E: 'static,
        Map: Fn(F) -> NextFinding + 'static,
    {
        let runner = self.runner.clone();
        let map = Arc::new(map);

        Step {
            runner: Arc::new(move |runtime, input| {
                let runner = runner.clone();
                let map = map.clone();
                Box::pin(async move {
                    match runner(runtime, input).await {
                        Ok(traced) => {
                            let (output, report) = traced.into_parts();
                            Ok(Traced::new(
                                output,
                                report.map_findings(|finding| map(finding)),
                            ))
                        }
                        Err(StepError::Rejected(report)) => Err(StepError::Rejected(
                            report.map_findings(|finding| map(finding)),
                        )),
                        Err(StepError::System { stage, error }) => {
                            Err(StepError::System { stage, error })
                        }
                    }
                })
            }),
        }
    }
}

impl<R, I, O, F, E> Clone for Step<R, I, O, F, E> {
    fn clone(&self) -> Self {
        Self {
            runner: self.runner.clone(),
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

impl<F, E> std::error::Error for StepError<F, E>
where
    F: Debug,
    E: std::error::Error + 'static,
{
}

impl<R, I, A, F, E> ValidationPipeline<R, I, A, A, F, E>
where
    R: 'static,
    I: 'static,
    A: 'static,
    F: 'static,
    E: 'static,
{
    fn identity() -> Self {
        Self {
            run: Arc::new(|_, _, output| Box::pin(async move { Ok((output, Vec::new())) })),
        }
    }
}

impl<R, I, A, S, F, E> ValidationPipeline<R, I, A, S, F, E>
where
    R: 'static,
    I: 'static,
    A: 'static,
    S: 'static,
    F: 'static,
    E: 'static,
{
    fn run<'a>(
        &'a self,
        runtime: &'a R,
        input: I,
        output: A,
    ) -> LocalBoxFuture<'a, Result<(S, Vec<F>), E>> {
        (self.run)(runtime, input, output)
    }

    fn bind_findings<NextFinding>(self) -> ValidationPipeline<R, I, A, S, NextFinding, E>
    where
        NextFinding: 'static,
    {
        let run = self.run.clone();

        ValidationPipeline {
            run: Arc::new(move |runtime, input, output| {
                let run = run.clone();
                Box::pin(async move {
                    let (subject, _) = run(runtime, input, output).await?;
                    Ok((subject, Vec::new()))
                })
            }),
        }
    }

    fn validate_first<C>(self, check: C) -> ValidationPipeline<R, I, A, S, C::Finding, E>
    where
        I: Clone + 'static,
        S: Clone + 'static,
        C: Check<Runtime = R, Input = I, Output = S, Error = E> + 'static,
        C::Finding: 'static,
    {
        let run = self.run.clone();
        let check = Arc::new(check);

        ValidationPipeline {
            run: Arc::new(move |runtime, input, output| {
                let run = run.clone();
                let check = check.clone();
                Box::pin(async move {
                    let (subject, _) = run(runtime, input.clone(), output).await?;
                    let findings = check.check(runtime, input, subject.clone()).await?;
                    Ok((subject, findings))
                })
            }),
        }
    }

    fn validate<C>(self, check: C) -> Self
    where
        I: Clone + 'static,
        S: Clone + 'static,
        C: Check<Runtime = R, Input = I, Output = S, Finding = F, Error = E> + 'static,
    {
        let run = self.run.clone();
        let check = Arc::new(check);

        Self {
            run: Arc::new(move |runtime, input, output| {
                let run = run.clone();
                let check = check.clone();
                Box::pin(async move {
                    let (subject, mut findings) = run(runtime, input.clone(), output).await?;
                    findings.extend(check.check(runtime, input, subject.clone()).await?);
                    Ok((subject, findings))
                })
            }),
        }
    }

    fn validate_into<C>(self, check: C) -> Self
    where
        I: Clone + 'static,
        S: Clone + 'static,
        C: Check<Runtime = R, Input = I, Output = S, Error = E> + 'static,
        C::Finding: Into<F> + 'static,
    {
        let run = self.run.clone();
        let check = Arc::new(check);

        Self {
            run: Arc::new(move |runtime, input, output| {
                let run = run.clone();
                let check = check.clone();
                Box::pin(async move {
                    let (subject, mut findings) = run(runtime, input.clone(), output).await?;
                    findings.extend(
                        check
                            .check(runtime, input, subject.clone())
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
    ) -> ValidationPipeline<R, I, A, NextSubject, F, E>
    where
        M: Materialiser<Runtime = R, Input = S, Output = NextSubject, Error = E> + 'static,
        NextSubject: 'static,
    {
        let run = self.run.clone();
        let materialiser = Arc::new(materialiser);

        ValidationPipeline {
            run: Arc::new(move |runtime, input, output| {
                let run = run.clone();
                let materialiser = materialiser.clone();
                Box::pin(async move {
                    let (subject, findings) = run(runtime, input, output).await?;
                    let next_subject = materialiser.materialise(runtime, subject).await?;
                    Ok((next_subject, findings))
                })
            }),
        }
    }
}

impl<R, I, A, S, F, E> Clone for ValidationPipeline<R, I, A, S, F, E> {
    fn clone(&self) -> Self {
        Self {
            run: self.run.clone(),
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
    ///
    /// The step output is the final validation subject. If the pipeline includes
    /// materialisation, this is the materialised artifact rather than the raw task
    /// output.
    pub fn build(self) -> Step<R, I, S, F, E>
    where
        I: Clone + 'static,
        A: Clone + 'static,
        S: 'static,
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
                        output_type = %type_name::<S>(),
                        finding_type = %type_name::<F>(),
                        max_attempts = ?retry_policy.max_attempts()
                    )
                } else {
                    info_span!(
                        name::STEP,
                        component = component::STEP,
                        task = task_name,
                        input_type = %type_name::<I>(),
                        output_type = %type_name::<S>(),
                        finding_type = %type_name::<F>(),
                        max_attempts = ?retry_policy.max_attempts()
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

                            let output =
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
                                attempt, "task produced output"
                            );

                            let (subject, findings): (S, Vec<F>) = pipeline
                                .run(runtime, input.clone(), output.clone())
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
                                return Ok(Traced::new(subject, StepReport::new(report_attempts)));
                            }

                            repair_attempts.push(Attempt {
                                input: input.clone(),
                                output: output.clone(),
                                findings,
                            });

                            if retry_policy.is_exhausted(repair_attempts.len()) {
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
    /// resume. The input, output, and finding types must be serialisable and
    /// deserialisable so their values can be persisted between sessions.
    pub fn build_persistent(self) -> Step<R, I, S, F, E>
    where
        I: Clone + Serialize + DeserializeOwned + 'static,
        A: Clone + Serialize + DeserializeOwned + 'static,
        S: Clone + Serialize + DeserializeOwned + 'static,
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
                        output_type = %type_name::<S>(),
                        finding_type = %type_name::<F>(),
                        max_attempts = ?retry_policy.max_attempts()
                    )
                } else {
                    info_span!(
                        name::STEP,
                        component = component::STEP,
                        task = task_name,
                        input_type = %type_name::<I>(),
                        output_type = %type_name::<S>(),
                        finding_type = %type_name::<F>(),
                        max_attempts = ?retry_policy.max_attempts()
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
                                            let output: A = serde_json::from_value(ac.output)
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
                                                output,
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

                            let output =
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
                                attempt, "task produced output"
                            );

                            let (subject, findings): (S, Vec<F>) = pipeline
                                .run(runtime, input.clone(), output.clone())
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
                                return Ok(Traced::new(subject, StepReport::new(report_attempts)));
                            }

                            repair_attempts.push(Attempt {
                                input: input.clone(),
                                output: output.clone(),
                                findings,
                            });

                            if retry_policy.is_exhausted(repair_attempts.len()) {
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
        I: Clone + 'static,
        S: Clone + 'static,
        C: Check<Runtime = R, Input = I, Output = S, Error = E> + 'static,
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
        I: Clone + 'static,
        S: Clone + 'static,
        C: Check<Runtime = R, Input = I, Output = S, Finding = F, Error = E> + 'static,
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
        I: Clone + 'static,
        S: Clone + 'static,
        C: Check<Runtime = R, Input = I, Output = S, Error = E> + 'static,
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
        P: RepairPlanner<Runtime = R, Input = I, Output = A, Finding = F, Error = E> + 'static,
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
            output: serde_json::to_value(&a.output).unwrap_or(Value::Null),
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
    enum WorkflowFinding {
        Step(Finding),
    }

    impl From<Finding> for WorkflowFinding {
        fn from(finding: Finding) -> Self {
            Self::Step(finding)
        }
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
        type Input = CodeInput;
        type Output = Patch;
        type Finding = Finding;
        type Error = TestError;

        fn check<'a>(
            &'a self,
            runtime: &'a Self::Runtime,
            _input: Self::Input,
            output: Self::Output,
        ) -> LocalBoxFuture<'a, Result<Vec<Self::Finding>, Self::Error>> {
            Box::pin(async move {
                if runtime.require_even_revision && output.revision % 2 != 0 {
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
        type Input = CodeInput;
        type Output = TestWorktree;
        type Finding = Finding;
        type Error = TestError;

        fn check<'a>(
            &'a self,
            _runtime: &'a Self::Runtime,
            _input: Self::Input,
            output: Self::Output,
        ) -> LocalBoxFuture<'a, Result<Vec<Self::Finding>, Self::Error>> {
            Box::pin(async move {
                if output.patch.passes_tests && output.patch.revision >= output.required_revision {
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
        type Output = Patch;
        type Finding = Finding;
        type Error = TestError;

        fn repair<'a>(
            &'a self,
            runtime: &'a Self::Runtime,
            attempts: Vec<Attempt<Self::Input, Self::Output, Self::Finding>>,
        ) -> LocalBoxFuture<'a, Result<Self::Input, Self::Error>> {
            Box::pin(async move {
                let previous = attempts.last().expect("attempt present");
                Ok(CodeInput {
                    prompt: previous.input.prompt,
                    revision: previous.output.revision + runtime.repair_increment,
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

    fn runtime() -> TestRuntime {
        TestRuntime {
            required_revision: 2,
            repair_increment: 1,
            require_even_revision: true,
            increment: 1,
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
            TestWorktree {
                patch: Patch {
                    revision: 2,
                    passes_tests: true,
                },
                required_revision: 2,
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

        assert_eq!(traced.output().patch.revision, 2);
        assert_eq!(traced.report().attempt_count(), 3);
        assert!(!traced.report().attempts()[0].accepted());
        assert!(!traced.report().attempts()[1].accepted());
        assert!(traced.report().attempts()[2].accepted());
        assert!(traced.report().attempts()[2].findings.is_empty());
    }

    #[tokio::test]
    async fn step_with_unlimited_retries_keeps_repairing_until_checks_pass() {
        let step = Step::builder(Generator)
            .materialise(CargoTest)
            .validate(TestCheck)
            .repair_with(Repair)
            .retry_policy(RetryPolicy::unlimited())
            .build();

        let mut test_runtime = runtime();
        test_runtime.required_revision = 5;
        test_runtime.require_even_revision = false;

        let traced = step
            .run_traced(
                &test_runtime,
                CodeInput {
                    prompt: "add feature",
                    revision: 0,
                    failing_test: None,
                },
            )
            .await
            .expect("unlimited retry policy should keep repairing");

        assert_eq!(traced.output().patch.revision, 5);
        assert_eq!(traced.report().attempt_count(), 6);
        assert!(traced.report().attempts()[5].accepted());
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
    async fn steps_map_input_output_and_findings() {
        let add_one = Step::builder(Increment).with_findings::<Finding>().build();

        let mapped_input = add_one.clone().map_input(|input: String| {
            input
                .parse::<usize>()
                .expect("test input should parse as usize")
        });
        let with_input = add_one.clone().with_input();
        let mapped_with_input =
            add_one.map_with_input(|input, output| format!("{input}->{output}"));
        let mapped_findings = Step::builder(Generator)
            .materialise(CargoTest)
            .validate(TestCheck)
            .retry_policy(RetryPolicy::new(1))
            .build()
            .map_findings(WorkflowFinding::from);
        let test_runtime = runtime();

        assert_eq!(
            mapped_input
                .run(&test_runtime, "3".to_string())
                .await
                .expect("mapped input result"),
            4
        );
        assert_eq!(
            with_input
                .run(&test_runtime, 3)
                .await
                .expect("with input result"),
            (3, 4)
        );
        assert_eq!(
            mapped_with_input
                .run(&test_runtime, 3)
                .await
                .expect("mapped with input result"),
            "3->4"
        );

        let error = mapped_findings
            .run(
                &test_runtime,
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
                assert_eq!(
                    report.attempts()[0].findings,
                    vec![WorkflowFinding::Step(Finding::TestFailure("cargo test"))]
                );
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
