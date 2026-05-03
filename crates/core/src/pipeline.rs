use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt::{Debug, Formatter},
    future::Future,
    pin::Pin,
    sync::Arc,
};

use futures::future::{LocalBoxFuture, try_join_all};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use thiserror::Error;
use tracing::{Instrument, debug, info, info_span, trace, warn};

use crate::{
    checkpoint::{PipelineCheckpoint, PipelineCheckpointer},
    span::{action, component, name, reason},
    step::{Step, StepError},
};

/// Unique identifier for one phase within a pipeline.
#[derive(
    Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct PhaseId(String);

impl PhaseId {
    /// Creates a phase identifier from a string.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the raw string identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PhaseId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for PhaseId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for PhaseId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

/// A typed, async unit of work within a pipeline.
///
/// Each phase declares its input and output types. The pipeline validates
/// route compatibility at construction time so that a phase's output type
/// matches the input type of the phase it routes to.
pub trait Phase {
    /// Shared runtime capabilities available to the phase.
    type Runtime;
    /// Input expected by this phase.
    type Input: 'static;
    /// Output produced by this phase.
    type Output: 'static;
    /// Errors returned by the phase logic.
    type Error;

    /// Executes the phase.
    fn run<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        input: Self::Input,
    ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>>;
}

/// Declares where control flows after a phase completes.
#[derive(Clone)]
#[allow(clippy::type_complexity)]
pub enum Route {
    /// Continue to the next phase.
    Next(PhaseId),
    /// Choose the next phase based on the current output.
    Switch(SwitchRoute),
    /// Run multiple phases concurrently; each receives the same input.
    Parallel(Vec<PhaseId>),
    /// Stop execution.
    Halt,
}

/// A conditional route with declared possible targets.
#[derive(Clone)]
pub struct SwitchRoute {
    output_type: TypeId,
    targets: Vec<PhaseId>,
    resolver: SwitchResolver,
}

type SwitchResolver = Arc<dyn Fn(&dyn Any) -> Option<PhaseId> + Send + Sync>;

impl Debug for Route {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Next(id) => write!(f, "Next({id})"),
            Self::Switch(route) => write!(f, "Switch({:?})", route.targets),
            Self::Parallel(ids) => {
                write!(f, "Parallel([")?;
                for (i, id) in ids.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{id}")?;
                }
                write!(f, "])")
            }
            Self::Halt => f.write_str("Halt"),
        }
    }
}

impl Route {
    /// Routes to a single next phase.
    pub fn next(phase_id: impl Into<PhaseId>) -> Self {
        Self::Next(phase_id.into())
    }

    /// Routes conditionally using a typed closure.
    ///
    /// The closure receives a reference to the phase output and must return
    /// the identifier of the phase to run next.
    pub fn switch<O, F>(targets: impl IntoIterator<Item = impl Into<PhaseId>>, f: F) -> Self
    where
        O: 'static,
        F: Fn(&O) -> PhaseId + Send + Sync + 'static,
    {
        Self::Switch(SwitchRoute {
            output_type: TypeId::of::<O>(),
            targets: targets.into_iter().map(Into::into).collect(),
            resolver: Arc::new(move |any| any.downcast_ref::<O>().map(&f)),
        })
    }

    /// Runs multiple phases in parallel.
    pub fn parallel(phase_ids: impl IntoIterator<Item = impl Into<PhaseId>>) -> Self {
        Self::Parallel(phase_ids.into_iter().map(Into::into).collect())
    }

    /// Halts pipeline execution.
    pub fn halt() -> Self {
        Self::Halt
    }
}

/// Errors that can occur while running a pipeline.
#[derive(Debug, Error)]
pub enum PipelineError<E> {
    /// A phase returned a domain or infrastructure error.
    #[error("phase '{phase_id}' failed: {error}")]
    Phase {
        phase_id: PhaseId,
        #[source]
        error: E,
    },
    /// A referenced phase does not exist in the pipeline.
    #[error("phase '{0}' not found in pipeline")]
    PhaseNotFound(PhaseId),
    /// A route referenced a phase that does not exist.
    #[error("route from '{from}' targets unknown phase '{to}'")]
    UnknownRouteTarget { from: PhaseId, to: PhaseId },
    /// A cycle was detected and the maximum depth was exceeded.
    #[error("maximum cycle depth exceeded at phase '{0}'")]
    MaxDepthExceeded(PhaseId),
    /// Type mismatch while passing output between phases.
    #[error("type mismatch passing output from '{from}' to '{to}'")]
    TypeMismatch { from: PhaseId, to: PhaseId },
    /// A parallel branch returned a non-halt route.
    #[error("parallel branch '{phase_id}' returned route '{route:?}' instead of Halt")]
    ParallelBranchNotHalted { phase_id: PhaseId, route: Route },
    /// A switch function referenced an unknown phase.
    #[error("switch from '{from}' selected unknown phase '{to}'")]
    SwitchUnknownTarget { from: PhaseId, to: PhaseId },
    /// Pipeline checkpoint serialisation failed.
    #[error("checkpoint for phase '{phase_id}' could not be serialised: {error}")]
    CheckpointSerialisation {
        phase_id: PhaseId,
        #[source]
        error: serde_json::Error,
    },
    /// Pipeline checkpoint deserialisation failed.
    #[error("checkpoint for phase '{phase_id}' could not be deserialised: {error}")]
    CheckpointDeserialisation {
        phase_id: PhaseId,
        #[source]
        error: serde_json::Error,
    },
    /// A phase was not registered with checkpoint serialisation support.
    #[error("phase '{0}' does not support pipeline checkpointing")]
    CheckpointUnsupported(PhaseId),
    /// Pipeline checkpoint persistence failed.
    #[error("pipeline checkpoint persistence failed: {0}")]
    CheckpointPersistence(Box<dyn std::error::Error + Send + Sync + 'static>),
    /// A step wrapped inside a phase was rejected.
    #[error("step in phase '{phase_id}' was rejected")]
    StepRejected { phase_id: PhaseId },
}

/// Internal type-erased phase runner.
type PhaseRunner<R, E> = Arc<
    dyn Fn(
        &R,
        Arc<dyn Any + Send + Sync>,
    ) -> LocalBoxFuture<'_, Result<Arc<dyn Any + Send + Sync>, PipelineError<E>>>,
>;

/// Internal type-erased route resolver.
type RouteResolver = Arc<dyn Fn(&dyn Any) -> Route>;
type CheckpointSerialiser<E> =
    Arc<dyn Fn(&dyn Any, &PhaseId) -> Result<Value, PipelineError<E>> + Send + Sync>;
type CheckpointRestorer<E> = Arc<
    dyn Fn(Value, &PhaseId) -> Result<Arc<dyn Any + Send + Sync>, PipelineError<E>> + Send + Sync,
>;
type ParallelJoiner<E> = Arc<
    dyn Fn(
            Vec<Arc<dyn Any + Send + Sync>>,
            &PhaseId,
        ) -> Result<Arc<dyn Any + Send + Sync>, PipelineError<E>>
        + Send
        + Sync,
>;
type PipelineRunFuture<'a, T, E> = Pin<Box<dyn Future<Output = Result<T, PipelineError<E>>> + 'a>>;
type PipelineResumeFuture<'a, T, E> =
    Pin<Box<dyn Future<Output = Result<Option<T>, PipelineError<E>>> + 'a>>;

/// Metadata stored for each registered phase.
struct PhaseEntry<R, E> {
    runner: PhaseRunner<R, E>,
    resolve_route: RouteResolver,
    serialise_checkpoint: CheckpointSerialiser<E>,
    restore_checkpoint: CheckpointRestorer<E>,
    join_parallel_outputs: ParallelJoiner<E>,
    input_type: TypeId,
    output_type: TypeId,
    joined_output_type: TypeId,
}

impl<R, E> Clone for PhaseEntry<R, E> {
    fn clone(&self) -> Self {
        Self {
            runner: self.runner.clone(),
            resolve_route: self.resolve_route.clone(),
            serialise_checkpoint: self.serialise_checkpoint.clone(),
            restore_checkpoint: self.restore_checkpoint.clone(),
            join_parallel_outputs: self.join_parallel_outputs.clone(),
            input_type: self.input_type,
            output_type: self.output_type,
            joined_output_type: self.joined_output_type,
        }
    }
}

/// A state-machine orchestrator that composes typed phases with declared routes.
///
/// `Pipeline` supports sequential execution, conditional branching, parallel
/// fan-out, and cycles (with a configurable depth limit). Each phase can wrap
/// a [`Step`] internally to reuse retry and repair machinery.
///
/// # Example
///
/// ```
/// use naaf_core::{Phase, PhaseId, Pipeline, Route};
/// use futures::future::LocalBoxFuture;
///
/// #[derive(Debug)]
/// struct Runtime;
///
/// #[derive(Clone)]
/// struct Increment;
/// impl Phase for Increment {
///     type Runtime = Runtime;
///     type Input = usize;
///     type Output = usize;
///     type Error = ();
///
///     fn run<'a>(&'a self, _rt: &'a Runtime, input: usize) -> LocalBoxFuture<'a, Result<usize, ()>> {
///         Box::pin(async move { Ok(input + 1) })
///     }
/// }
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let pipeline = Pipeline::builder()
///     .add_phase(PhaseId::new("inc"), Increment)
///     .with_route(PhaseId::new("inc"), Route::Halt)
///     .with_initial(PhaseId::new("inc"))
///     .build()
///     .unwrap();
///
/// let result: usize = pipeline.run(&Runtime, 5usize).await.unwrap();
/// assert_eq!(result, 6);
/// # });
/// ```
pub struct Pipeline<R, E> {
    phases: HashMap<PhaseId, PhaseEntry<R, E>>,
    parallel_join_routes: HashMap<PhaseId, PhaseId>,
    initial_phase: PhaseId,
    max_cycle_depth: usize,
    checkpointer: Option<Arc<dyn PipelineCheckpointer>>,
}

/// Builder for constructing a [`Pipeline`].
pub struct PipelineBuilder<R, E> {
    phases: HashMap<PhaseId, PhaseEntry<R, E>>,
    routes: HashMap<PhaseId, Route>,
    parallel_join_routes: HashMap<PhaseId, PhaseId>,
    initial_phase: Option<PhaseId>,
    max_cycle_depth: usize,
    checkpointer: Option<Arc<dyn PipelineCheckpointer>>,
}

impl<R, E> Default for PipelineBuilder<R, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R, E> PipelineBuilder<R, E> {
    /// Creates a new, empty pipeline builder.
    pub fn new() -> Self {
        Self {
            phases: HashMap::new(),
            routes: HashMap::new(),
            parallel_join_routes: HashMap::new(),
            initial_phase: None,
            max_cycle_depth: 64,
            checkpointer: None,
        }
    }

    /// Registers a typed phase under the given identifier.
    pub fn add_phase<P>(mut self, id: PhaseId, phase: P) -> Self
    where
        P: Phase<Runtime = R, Error = E> + Clone + 'static,
        P::Input: Clone + Send + Sync,
        P::Output: Clone + Send + Sync,
    {
        let input_type = TypeId::of::<P::Input>();
        let output_type = TypeId::of::<P::Output>();
        let joined_output_type = TypeId::of::<Vec<P::Output>>();

        let runner: PhaseRunner<R, E> = Arc::new(move |runtime, input| {
            let phase = phase.clone();
            Box::pin(async move {
                let typed_input =
                    Arc::downcast::<P::Input>(input).map_err(|_| PipelineError::TypeMismatch {
                        from: PhaseId::new("unknown"),
                        to: PhaseId::new("unknown"),
                    })?;
                let typed_input = (*typed_input).clone();
                let output = phase.run(runtime, typed_input).await.map_err(|error| {
                    PipelineError::Phase {
                        phase_id: PhaseId::new("unknown"),
                        error,
                    }
                })?;
                Ok(Arc::new(output) as Arc<dyn Any + Send + Sync>)
            })
        });

        let resolve_route: RouteResolver = Arc::new(|_| Route::Halt);
        let serialise_checkpoint: CheckpointSerialiser<E> =
            Arc::new(|_, phase_id| Err(PipelineError::CheckpointUnsupported(phase_id.clone())));
        let restore_checkpoint: CheckpointRestorer<E> =
            Arc::new(|_, phase_id| Err(PipelineError::CheckpointUnsupported(phase_id.clone())));
        let join_parallel_outputs: ParallelJoiner<E> = Arc::new(|outputs, phase_id| {
            let mut joined = Vec::with_capacity(outputs.len());
            for output in outputs {
                let output = Arc::downcast::<P::Output>(output).map_err(|_| {
                    PipelineError::TypeMismatch {
                        from: phase_id.clone(),
                        to: PhaseId::new("parallel-join"),
                    }
                })?;
                joined.push((*output).clone());
            }
            Ok(Arc::new(joined) as Arc<dyn Any + Send + Sync>)
        });

        self.phases.insert(
            id,
            PhaseEntry {
                runner,
                resolve_route,
                serialise_checkpoint,
                restore_checkpoint,
                join_parallel_outputs,
                input_type,
                output_type,
                joined_output_type,
            },
        );
        self
    }

    /// Registers a typed phase with pipeline checkpoint serialisation support.
    pub fn add_persistent_phase<P>(self, id: PhaseId, phase: P) -> Self
    where
        P: Phase<Runtime = R, Error = E> + Clone + 'static,
        P::Input: Clone + Send + Sync,
        P::Output: Clone + Send + Sync + Serialize + DeserializeOwned,
    {
        self.add_phase(id.clone(), phase)
            .with_phase_checkpoint::<P::Output>(id)
    }

    /// Adds a phase that wraps a [`Step`].
    ///
    /// The step's output becomes the phase output, and its [`StepReport`] is
    /// discarded. If you need the report, use [`PipelineBuilder::add_phase`]
    /// with a custom phase implementation.
    pub fn add_step<I, O, F>(mut self, id: PhaseId, step: Step<R, I, O, F, E>) -> Self
    where
        R: 'static,
        I: Clone + Send + Sync + 'static,
        O: Clone + Send + Sync + 'static,
        F: 'static,
        E: 'static,
    {
        let input_type = TypeId::of::<I>();
        let output_type = TypeId::of::<O>();
        let joined_output_type = TypeId::of::<Vec<O>>();

        let runner: PhaseRunner<R, E> = Arc::new(move |runtime, input| {
            let step = step.clone();
            Box::pin(async move {
                let typed_input =
                    Arc::downcast::<I>(input).map_err(|_| PipelineError::TypeMismatch {
                        from: PhaseId::new("unknown"),
                        to: PhaseId::new("unknown"),
                    })?;
                let typed_input = (*typed_input).clone();
                let output = step
                    .run(runtime, typed_input)
                    .await
                    .map_err(|error| match error {
                        StepError::System { error, .. } => PipelineError::Phase {
                            phase_id: PhaseId::new("unknown"),
                            error,
                        },
                        StepError::Rejected(_) => PipelineError::StepRejected {
                            phase_id: PhaseId::new("unknown"),
                        },
                    })?;
                Ok(Arc::new(output) as Arc<dyn Any + Send + Sync>)
            })
        });

        let resolve_route: RouteResolver = Arc::new(|_| Route::Halt);
        let serialise_checkpoint: CheckpointSerialiser<E> =
            Arc::new(|_, phase_id| Err(PipelineError::CheckpointUnsupported(phase_id.clone())));
        let restore_checkpoint: CheckpointRestorer<E> =
            Arc::new(|_, phase_id| Err(PipelineError::CheckpointUnsupported(phase_id.clone())));
        let join_parallel_outputs: ParallelJoiner<E> = Arc::new(|outputs, phase_id| {
            let mut joined = Vec::with_capacity(outputs.len());
            for output in outputs {
                let output =
                    Arc::downcast::<O>(output).map_err(|_| PipelineError::TypeMismatch {
                        from: phase_id.clone(),
                        to: PhaseId::new("parallel-join"),
                    })?;
                joined.push((*output).clone());
            }
            Ok(Arc::new(joined) as Arc<dyn Any + Send + Sync>)
        });

        self.phases.insert(
            id,
            PhaseEntry {
                runner,
                resolve_route,
                serialise_checkpoint,
                restore_checkpoint,
                join_parallel_outputs,
                input_type,
                output_type,
                joined_output_type,
            },
        );
        self
    }

    /// Adds a Step-backed phase with pipeline checkpoint serialisation support.
    pub fn add_persistent_step<I, O, F>(self, id: PhaseId, step: Step<R, I, O, F, E>) -> Self
    where
        R: 'static,
        I: Clone + Send + Sync + 'static,
        O: Clone + Send + Sync + Serialize + DeserializeOwned + 'static,
        F: 'static,
        E: 'static,
    {
        self.add_step(id.clone(), step)
            .with_phase_checkpoint::<O>(id)
    }

    fn with_phase_checkpoint<O>(mut self, id: PhaseId) -> Self
    where
        O: Clone + Send + Sync + Serialize + DeserializeOwned + 'static,
    {
        if let Some(entry) = self.phases.get_mut(&id) {
            entry.serialise_checkpoint = Arc::new(|output, phase_id| {
                let output =
                    output
                        .downcast_ref::<O>()
                        .ok_or_else(|| PipelineError::TypeMismatch {
                            from: phase_id.clone(),
                            to: PhaseId::new("checkpoint"),
                        })?;
                serde_json::to_value(output).map_err(|error| {
                    PipelineError::CheckpointSerialisation {
                        phase_id: phase_id.clone(),
                        error,
                    }
                })
            });
            entry.restore_checkpoint = Arc::new(|value, phase_id| {
                let output: O = serde_json::from_value(value).map_err(|error| {
                    PipelineError::CheckpointDeserialisation {
                        phase_id: phase_id.clone(),
                        error,
                    }
                })?;
                Ok(Arc::new(output) as Arc<dyn Any + Send + Sync>)
            });
        }
        self
    }

    /// Sets the route returned by a phase after it completes.
    pub fn with_route(mut self, id: PhaseId, route: Route) -> Self {
        self.routes.insert(id, route);
        self
    }

    /// Sets the phase that receives joined outputs from a `Route::Parallel` fan-out.
    pub fn with_parallel_join(
        mut self,
        parallel_phase: impl Into<PhaseId>,
        join_phase: impl Into<PhaseId>,
    ) -> Self {
        self.parallel_join_routes
            .insert(parallel_phase.into(), join_phase.into());
        self
    }

    /// Sets the initial phase for the pipeline.
    pub fn with_initial(mut self, id: PhaseId) -> Self {
        self.initial_phase = Some(id);
        self
    }

    /// Sets the maximum cycle depth before erroring.
    pub fn with_max_cycle_depth(mut self, depth: usize) -> Self {
        self.max_cycle_depth = depth;
        self
    }

    /// Installs a checkpointer that receives a checkpoint after each phase completes.
    pub fn checkpoint_with(mut self, checkpointer: impl PipelineCheckpointer + 'static) -> Self {
        self.checkpointer = Some(Arc::new(checkpointer));
        self
    }

    /// Consumes the builder and produces a validated [`Pipeline`].
    ///
    /// Validates that:
    /// - An initial phase is set.
    /// - The initial phase exists.
    /// - Every route target exists as a registered phase.
    /// - For `Next` and `Parallel` routes, the source's output type matches
    ///   the target's input type.
    pub fn build(self) -> Result<Pipeline<R, E>, PipelineValidationError> {
        let initial_phase = self
            .initial_phase
            .ok_or(PipelineValidationError::MissingInitialPhase)?;

        if !self.phases.contains_key(&initial_phase) {
            return Err(PipelineValidationError::InitialPhaseNotFound(initial_phase));
        }

        // Validate route targets exist and type-match.
        for (from_id, route) in &self.routes {
            let from_entry = self
                .phases
                .get(from_id)
                .ok_or_else(|| PipelineValidationError::PhaseNotFound(from_id.clone()))?;

            let target_ids: Vec<&PhaseId> = match route {
                Route::Next(to) => vec![to],
                Route::Parallel(tos) => tos.iter().collect(),
                Route::Switch(switch) => switch.targets.iter().collect(),
                Route::Halt => continue,
            };

            if let Route::Switch(switch) = route
                && from_entry.output_type != switch.output_type
            {
                return Err(PipelineValidationError::TypeMismatch {
                    from: from_id.clone(),
                    to: PhaseId::new("switch"),
                });
            }

            let mut parallel_output_type = None;
            for to_id in target_ids {
                let to_entry = self.phases.get(to_id).ok_or_else(|| {
                    PipelineValidationError::UnknownRouteTarget {
                        from: from_id.clone(),
                        to: to_id.clone(),
                    }
                })?;

                if from_entry.output_type != to_entry.input_type {
                    return Err(PipelineValidationError::TypeMismatch {
                        from: from_id.clone(),
                        to: to_id.clone(),
                    });
                }

                if let Route::Parallel(_) = route
                    && parallel_output_type
                        .replace(to_entry.output_type)
                        .is_some_and(|expected| expected != to_entry.output_type)
                {
                    return Err(PipelineValidationError::TypeMismatch {
                        from: from_id.clone(),
                        to: to_id.clone(),
                    });
                }
            }
        }

        for (from_id, join_id) in &self.parallel_join_routes {
            let Some(Route::Parallel(branch_ids)) = self.routes.get(from_id) else {
                return Err(PipelineValidationError::PhaseNotFound(from_id.clone()));
            };
            let Some(first_branch_id) = branch_ids.first() else {
                return Err(PipelineValidationError::PhaseNotFound(from_id.clone()));
            };
            let first_branch = self.phases.get(first_branch_id).ok_or_else(|| {
                PipelineValidationError::UnknownRouteTarget {
                    from: from_id.clone(),
                    to: first_branch_id.clone(),
                }
            })?;
            let join_entry = self.phases.get(join_id).ok_or_else(|| {
                PipelineValidationError::UnknownRouteTarget {
                    from: from_id.clone(),
                    to: join_id.clone(),
                }
            })?;
            if first_branch.joined_output_type != join_entry.input_type {
                return Err(PipelineValidationError::TypeMismatch {
                    from: from_id.clone(),
                    to: join_id.clone(),
                });
            }
        }

        // Merge explicit routes into phase entries.
        let mut phases = self.phases;
        for (id, route) in self.routes {
            if let Some(entry) = phases.get_mut(&id) {
                entry.resolve_route = Arc::new(move |_output: &dyn Any| route.clone());
            }
        }

        Ok(Pipeline {
            phases,
            parallel_join_routes: self.parallel_join_routes,
            initial_phase,
            max_cycle_depth: self.max_cycle_depth,
            checkpointer: self.checkpointer,
        })
    }
}

/// Errors discovered while validating a pipeline at construction time.
#[derive(Debug, Error, PartialEq)]
pub enum PipelineValidationError {
    #[error("no initial phase configured")]
    MissingInitialPhase,
    #[error("initial phase '{0}' not found")]
    InitialPhaseNotFound(PhaseId),
    #[error("phase '{0}' not found")]
    PhaseNotFound(PhaseId),
    #[error("route from '{from}' targets unknown phase '{to}'")]
    UnknownRouteTarget { from: PhaseId, to: PhaseId },
    #[error("type mismatch passing output from '{from}' to '{to}'")]
    TypeMismatch { from: PhaseId, to: PhaseId },
}

impl<R, E> Pipeline<R, E> {
    /// Creates a new pipeline builder.
    pub fn builder() -> PipelineBuilder<R, E> {
        PipelineBuilder::new()
    }

    /// Runs the pipeline from the initial phase with the given input.
    ///
    /// The input type `I` must match the initial phase's declared input type.
    /// The output type `O` must match the type of the final phase that halts.
    pub fn run<'a, I, O>(&'a self, runtime: &'a R, input: I) -> PipelineRunFuture<'a, O, E>
    where
        I: Clone + Send + Sync + 'static,
        O: Clone + Send + Sync + 'static,
    {
        let input = Arc::new(input) as Arc<dyn Any + Send + Sync>;
        self.run_inner(
            runtime,
            self.initial_phase.clone(),
            input,
            None,
            HashMap::new(),
            0,
        )
    }

    /// Resumes pipeline execution from a checkpoint saved after a phase completed.
    pub fn resume<'a, O>(&'a self, runtime: &'a R) -> PipelineResumeFuture<'a, O, E>
    where
        O: Clone + Send + Sync + 'static,
    {
        Box::pin(async move {
            let Some(checkpointer) = &self.checkpointer else {
                return Ok(None);
            };
            let checkpoint = checkpointer
                .load_pipeline()
                .await
                .map_err(PipelineError::CheckpointPersistence)?;
            let Some(checkpoint) = checkpoint else {
                return Ok(None);
            };
            self.run_from_checkpoint(runtime, checkpoint)
                .await
                .map(Some)
        })
    }

    /// Resumes pipeline execution from a checkpoint value saved after a phase completed.
    pub fn run_from_checkpoint<'a, O>(
        &'a self,
        runtime: &'a R,
        checkpoint: PipelineCheckpoint,
    ) -> PipelineRunFuture<'a, O, E>
    where
        O: Clone + Send + Sync + 'static,
    {
        Box::pin(async move {
            let entry = self
                .phases
                .get(&checkpoint.current_phase)
                .ok_or_else(|| PipelineError::PhaseNotFound(checkpoint.current_phase.clone()))?;
            let output =
                (entry.restore_checkpoint)(checkpoint.phase_output, &checkpoint.current_phase)?;
            self.run_inner(
                runtime,
                checkpoint.current_phase,
                Arc::clone(&output),
                Some(output),
                checkpoint.phase_visits,
                checkpoint.completed_phases,
            )
            .await
        })
    }

    fn run_inner<'a, O>(
        &'a self,
        runtime: &'a R,
        start_phase: PhaseId,
        input: Arc<dyn Any + Send + Sync>,
        resume_output: Option<Arc<dyn Any + Send + Sync>>,
        mut phase_visits: HashMap<PhaseId, usize>,
        mut completed_phases: usize,
    ) -> PipelineRunFuture<'a, O, E>
    where
        O: Clone + Send + Sync + 'static,
    {
        Box::pin(async move {
            let mut current_phase = start_phase;
            let mut current_input = input;
            let mut resume_output = resume_output;

            let pipeline_span = info_span!(
                name::PIPELINE,
                component = component::PIPELINE,
                initial_phase = %current_phase,
                max_cycle_depth = self.max_cycle_depth,
            );

            async move {
                info!(action = action::RUN_START, "pipeline started");

                loop {
                    let entry = self
                        .phases
                        .get(&current_phase)
                        .ok_or_else(|| PipelineError::PhaseNotFound(current_phase.clone()))?;

                    let output = if let Some(output) = resume_output.take() {
                        output
                    } else {
                        let visits = phase_visits.entry(current_phase.clone()).or_insert(0);
                        *visits += 1;
                        if *visits > self.max_cycle_depth {
                            warn!(
                                action = action::RUN_REJECTED,
                                phase = %current_phase,
                                depth = *visits,
                                reason = reason::MAX_DEPTH_EXCEEDED,
                                "pipeline max cycle depth exceeded"
                            );
                            return Err(PipelineError::MaxDepthExceeded(current_phase));
                        }

                        trace!(
                            action = action::ATTEMPT_START,
                            phase = %current_phase,
                            depth = *visits,
                            "phase started"
                        );

                        let output = (entry.runner)(runtime, current_input).await.map_err(|e| {
                            if let PipelineError::Phase { error, .. } = e {
                                PipelineError::Phase {
                                    phase_id: current_phase.clone(),
                                    error,
                                }
                            } else if let PipelineError::StepRejected { .. } = e {
                                PipelineError::StepRejected {
                                    phase_id: current_phase.clone(),
                                }
                            } else {
                                e
                            }
                        })?;
                        completed_phases += 1;
                        self.save_checkpoint(
                            entry,
                            &current_phase,
                            Arc::clone(&output),
                            completed_phases,
                            &phase_visits,
                        )
                        .await?;
                        output
                    };

                    trace!(
                        action = action::ATTEMPT_OUTPUT,
                        phase = %current_phase,
                        "phase completed"
                    );

                    let route = (entry.resolve_route)(output.as_ref());

                    debug!(
                        action = action::ROUTE,
                        phase = %current_phase,
                        route = ?route,
                        "route selected"
                    );

                    match route {
                        Route::Halt => {
                            info!(
                                action = action::RUN_COMPLETE,
                                phase = %current_phase,
                                "pipeline halted"
                            );
                            let final_output = Arc::downcast::<O>(output).map_err(|_| {
                                PipelineError::TypeMismatch {
                                    from: current_phase.clone(),
                                    to: PhaseId::new("halt"),
                                }
                            })?;
                            return Ok((*final_output).clone());
                        }
                        Route::Next(next_id) => {
                            let next_entry = self.phases.get(&next_id).ok_or_else(|| {
                                PipelineError::UnknownRouteTarget {
                                    from: current_phase.clone(),
                                    to: next_id.clone(),
                                }
                            })?;

                            if entry.output_type != next_entry.input_type {
                                return Err(PipelineError::TypeMismatch {
                                    from: current_phase.clone(),
                                    to: next_id.clone(),
                                });
                            }

                            current_phase = next_id;
                            current_input = output;
                        }
                        Route::Switch(switch) => {
                            let Some(next_id) = (switch.resolver)(output.as_ref()) else {
                                return Err(PipelineError::TypeMismatch {
                                    from: current_phase.clone(),
                                    to: PhaseId::new("switch"),
                                });
                            };
                            if !switch.targets.contains(&next_id) {
                                return Err(PipelineError::SwitchUnknownTarget {
                                    from: current_phase.clone(),
                                    to: next_id,
                                });
                            }
                            let next_entry = self.phases.get(&next_id).ok_or_else(|| {
                                PipelineError::SwitchUnknownTarget {
                                    from: current_phase.clone(),
                                    to: next_id.clone(),
                                }
                            })?;

                            if entry.output_type != next_entry.input_type {
                                return Err(PipelineError::TypeMismatch {
                                    from: current_phase.clone(),
                                    to: next_id.clone(),
                                });
                            }

                            current_phase = next_id;
                            current_input = output;
                        }
                        Route::Parallel(phase_ids) => {
                            let Some(first_phase) = phase_ids.first() else {
                                return Err(PipelineError::PhaseNotFound(current_phase.clone()));
                            };
                            let first_entry = self
                                .phases
                                .get(first_phase)
                                .ok_or_else(|| PipelineError::PhaseNotFound(first_phase.clone()))?;

                            let mut futures = Vec::with_capacity(phase_ids.len());
                            for pid in &phase_ids {
                                let entry = self
                                    .phases
                                    .get(pid)
                                    .cloned()
                                    .ok_or_else(|| PipelineError::PhaseNotFound(pid.clone()))?;
                                let pid = pid.clone();
                                let output = Arc::clone(&output);
                                let runtime = runtime;
                                let checkpointer = self.checkpointer.clone();
                                futures.push(async move {
                                    let branch_output =
                                        (entry.runner)(runtime, output).await.map_err(|e| {
                                            if let PipelineError::Phase { error, .. } = e {
                                                PipelineError::Phase {
                                                    phase_id: pid.clone(),
                                                    error,
                                                }
                                            } else if let PipelineError::StepRejected { .. } = e {
                                                PipelineError::StepRejected {
                                                    phase_id: pid.clone(),
                                                }
                                            } else {
                                                e
                                            }
                                        })?;

                                    let route = (entry.resolve_route)(branch_output.as_ref());
                                    if !matches!(route, Route::Halt) {
                                        return Err(PipelineError::ParallelBranchNotHalted {
                                            phase_id: pid.clone(),
                                            route,
                                        });
                                    }

                                    if let Some(checkpointer) = checkpointer {
                                        let phase_output = (entry.serialise_checkpoint)(
                                            branch_output.as_ref(),
                                            &pid,
                                        )?;
                                        checkpointer
                                            .save_pipeline(PipelineCheckpoint {
                                                current_phase: pid.clone(),
                                                phase_output,
                                                completed_phases: 1,
                                                phase_visits: HashMap::from([(pid.clone(), 1)]),
                                            })
                                            .await
                                            .map_err(PipelineError::CheckpointPersistence)?;
                                    }

                                    Ok::<_, PipelineError<E>>(branch_output)
                                });
                            }

                            let results = try_join_all(futures).await?;
                            let joined =
                                (first_entry.join_parallel_outputs)(results, &current_phase)?;

                            info!(
                                action = action::RUN_COMPLETE,
                                phase = %current_phase,
                                parallel_branches = phase_ids.len(),
                                "parallel branches joined"
                            );

                            if let Some(join_phase) =
                                self.parallel_join_routes.get(&current_phase).cloned()
                            {
                                current_phase = join_phase;
                                current_input = joined;
                            } else {
                                let final_output = Arc::downcast::<O>(joined).map_err(|_| {
                                    PipelineError::TypeMismatch {
                                        from: current_phase.clone(),
                                        to: PhaseId::new("halt"),
                                    }
                                })?;
                                return Ok((*final_output).clone());
                            }
                        }
                    }
                }
            }
            .instrument(pipeline_span)
            .await
        })
    }

    async fn save_checkpoint(
        &self,
        entry: &PhaseEntry<R, E>,
        phase_id: &PhaseId,
        output: Arc<dyn Any + Send + Sync>,
        completed_phases: usize,
        phase_visits: &HashMap<PhaseId, usize>,
    ) -> Result<(), PipelineError<E>> {
        let Some(checkpointer) = &self.checkpointer else {
            return Ok(());
        };
        let phase_output = (entry.serialise_checkpoint)(output.as_ref(), phase_id)?;
        checkpointer
            .save_pipeline(PipelineCheckpoint {
                current_phase: phase_id.clone(),
                phase_output,
                completed_phases,
                phase_visits: phase_visits.clone(),
            })
            .await
            .map_err(PipelineError::CheckpointPersistence)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        any::Any,
        sync::{
            Arc, Mutex,
            atomic::{AtomicUsize, Ordering},
        },
    };

    use futures::future::LocalBoxFuture;

    use super::{Phase, PhaseId, Pipeline, Route};
    use crate::{PipelineCheckpoint, PipelineCheckpointer, step::Step, task::Task};

    #[derive(Debug)]
    struct TestRuntime;

    // 1.9 Linear pipeline
    #[tokio::test]
    async fn linear_pipeline_runs_phases_in_order() {
        #[derive(Clone)]
        struct A;
        impl Phase for A {
            type Runtime = TestRuntime;
            type Input = usize;
            type Output = usize;
            type Error = ();
            fn run<'a>(
                &'a self,
                _rt: &'a TestRuntime,
                input: usize,
            ) -> LocalBoxFuture<'a, Result<usize, ()>> {
                Box::pin(async move { Ok(input + 1) })
            }
        }

        #[derive(Clone)]
        struct B;
        impl Phase for B {
            type Runtime = TestRuntime;
            type Input = usize;
            type Output = usize;
            type Error = ();
            fn run<'a>(
                &'a self,
                _rt: &'a TestRuntime,
                input: usize,
            ) -> LocalBoxFuture<'a, Result<usize, ()>> {
                Box::pin(async move { Ok(input * 2) })
            }
        }

        let pipeline = Pipeline::builder()
            .add_phase(PhaseId::new("a"), A)
            .add_phase(PhaseId::new("b"), B)
            .with_route(PhaseId::new("a"), Route::next("b"))
            .with_route(PhaseId::new("b"), Route::Halt)
            .with_initial(PhaseId::new("a"))
            .build()
            .unwrap();

        let result: usize = pipeline.run(&TestRuntime, 3usize).await.unwrap();
        assert_eq!(result, 8); // (3 + 1) * 2
    }

    // 1.9 Switch / conditional routing
    #[tokio::test]
    async fn switch_route_branches_conditionally() {
        #[derive(Clone)]
        struct Decide;
        impl Phase for Decide {
            type Runtime = TestRuntime;
            type Input = usize;
            type Output = usize;
            type Error = ();
            fn run<'a>(
                &'a self,
                _rt: &'a TestRuntime,
                input: usize,
            ) -> LocalBoxFuture<'a, Result<usize, ()>> {
                Box::pin(async move { Ok(input) })
            }
        }

        #[derive(Clone)]
        struct Increment;
        impl Phase for Increment {
            type Runtime = TestRuntime;
            type Input = usize;
            type Output = usize;
            type Error = ();
            fn run<'a>(
                &'a self,
                _rt: &'a TestRuntime,
                input: usize,
            ) -> LocalBoxFuture<'a, Result<usize, ()>> {
                Box::pin(async move { Ok(input + 1) })
            }
        }

        #[derive(Clone)]
        struct Decrement;
        impl Phase for Decrement {
            type Runtime = TestRuntime;
            type Input = usize;
            type Output = usize;
            type Error = ();
            fn run<'a>(
                &'a self,
                _rt: &'a TestRuntime,
                input: usize,
            ) -> LocalBoxFuture<'a, Result<usize, ()>> {
                Box::pin(async move { Ok(input - 1) })
            }
        }

        let pipeline = Pipeline::builder()
            .add_phase(PhaseId::new("decide"), Decide)
            .add_phase(PhaseId::new("inc"), Increment)
            .add_phase(PhaseId::new("dec"), Decrement)
            .with_route(
                PhaseId::new("decide"),
                Route::switch(["inc", "dec"], |n: &usize| {
                    if *n > 5 {
                        PhaseId::new("inc")
                    } else {
                        PhaseId::new("dec")
                    }
                }),
            )
            .with_route(PhaseId::new("inc"), Route::Halt)
            .with_route(PhaseId::new("dec"), Route::Halt)
            .with_initial(PhaseId::new("decide"))
            .build()
            .unwrap();

        let high: usize = pipeline.run(&TestRuntime, 10usize).await.unwrap();
        assert_eq!(high, 11);

        let low: usize = pipeline.run(&TestRuntime, 3usize).await.unwrap();
        assert_eq!(low, 2);
    }

    // 1.9 Parallel execution
    #[tokio::test]
    async fn parallel_runs_phases_concurrently() {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);

        #[derive(Clone)]
        struct Inc;
        impl Phase for Inc {
            type Runtime = TestRuntime;
            type Input = usize;
            type Output = usize;
            type Error = ();
            fn run<'a>(
                &'a self,
                _rt: &'a TestRuntime,
                input: usize,
            ) -> LocalBoxFuture<'a, Result<usize, ()>> {
                Box::pin(async move {
                    COUNTER.fetch_add(1, Ordering::SeqCst);
                    Ok(input + 1)
                })
            }
        }

        let pipeline = Pipeline::builder()
            .add_phase(PhaseId::new("start"), Inc)
            .add_phase(PhaseId::new("a"), Inc)
            .add_phase(PhaseId::new("b"), Inc)
            .add_phase(PhaseId::new("c"), Inc)
            .with_route(PhaseId::new("start"), Route::parallel(["a", "b", "c"]))
            .with_route(PhaseId::new("a"), Route::Halt)
            .with_route(PhaseId::new("b"), Route::Halt)
            .with_route(PhaseId::new("c"), Route::Halt)
            .with_initial(PhaseId::new("start"))
            .build()
            .unwrap();

        COUNTER.store(0, Ordering::SeqCst);
        let result: Vec<usize> = pipeline.run(&TestRuntime, 0usize).await.unwrap();
        // Parallel branches all receive the same input (1 from start) and return 2.
        assert_eq!(result, vec![2, 2, 2]);
        // All three parallel branches should have executed.
        assert_eq!(COUNTER.load(Ordering::SeqCst), 4); // start + a + b + c
    }

    #[derive(Clone, Default)]
    struct MemoryPipelineCheckpointer {
        checkpoints: Arc<Mutex<Vec<PipelineCheckpoint>>>,
    }

    impl PipelineCheckpointer for MemoryPipelineCheckpointer {
        fn save_pipeline(
            &self,
            checkpoint: PipelineCheckpoint,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::CheckpointResult<()>> + Send>>
        {
            self.checkpoints
                .lock()
                .expect("checkpoints lock")
                .push(checkpoint);
            Box::pin(async { Ok(()) })
        }

        fn load_pipeline(
            &self,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = crate::CheckpointResult<Option<PipelineCheckpoint>>,
                    > + Send,
            >,
        > {
            let checkpoint = self
                .checkpoints
                .lock()
                .expect("checkpoints lock")
                .last()
                .cloned();
            Box::pin(async move { Ok(checkpoint) })
        }
    }

    #[tokio::test]
    async fn checkpoints_after_phase_and_resumes_from_checkpoint() {
        #[derive(Clone)]
        struct AddOne;
        impl Phase for AddOne {
            type Runtime = TestRuntime;
            type Input = usize;
            type Output = usize;
            type Error = ();
            fn run<'a>(
                &'a self,
                _rt: &'a TestRuntime,
                input: usize,
            ) -> LocalBoxFuture<'a, Result<usize, ()>> {
                Box::pin(async move { Ok(input + 1) })
            }
        }

        #[derive(Clone)]
        struct Double;
        impl Phase for Double {
            type Runtime = TestRuntime;
            type Input = usize;
            type Output = usize;
            type Error = ();
            fn run<'a>(
                &'a self,
                _rt: &'a TestRuntime,
                input: usize,
            ) -> LocalBoxFuture<'a, Result<usize, ()>> {
                Box::pin(async move { Ok(input * 2) })
            }
        }

        let checkpointer = MemoryPipelineCheckpointer::default();
        let checkpoints = checkpointer.checkpoints.clone();
        let pipeline = Pipeline::builder()
            .add_persistent_phase(PhaseId::new("add"), AddOne)
            .add_persistent_phase(PhaseId::new("double"), Double)
            .with_route(PhaseId::new("add"), Route::next("double"))
            .with_route(PhaseId::new("double"), Route::Halt)
            .with_initial(PhaseId::new("add"))
            .checkpoint_with(checkpointer)
            .build()
            .unwrap();

        let result: usize = pipeline.run(&TestRuntime, 3usize).await.unwrap();
        assert_eq!(result, 8);
        let first_checkpoint = checkpoints.lock().expect("checkpoints lock")[0].clone();

        let resumed: usize = pipeline
            .run_from_checkpoint(&TestRuntime, first_checkpoint)
            .await
            .unwrap();
        assert_eq!(resumed, 8);
    }

    // 1.9 Cycle / max depth
    #[tokio::test]
    async fn cycle_hits_max_depth() {
        #[derive(Clone)]
        struct Loop;
        impl Phase for Loop {
            type Runtime = TestRuntime;
            type Input = usize;
            type Output = usize;
            type Error = ();
            fn run<'a>(
                &'a self,
                _rt: &'a TestRuntime,
                input: usize,
            ) -> LocalBoxFuture<'a, Result<usize, ()>> {
                Box::pin(async move { Ok(input + 1) })
            }
        }

        let pipeline = Pipeline::builder()
            .add_phase(PhaseId::new("loop"), Loop)
            .with_route(PhaseId::new("loop"), Route::next("loop"))
            .with_initial(PhaseId::new("loop"))
            .with_max_cycle_depth(3)
            .build()
            .unwrap();

        let err = pipeline
            .run::<usize, usize>(&TestRuntime, 0usize)
            .await
            .unwrap_err();
        assert!(matches!(err, super::PipelineError::MaxDepthExceeded(_)));
    }

    // 1.10 Step-wrapping phase
    #[tokio::test]
    async fn step_wrapped_in_phase() {
        struct Doubler;
        impl Task for Doubler {
            type Runtime = TestRuntime;
            type Input = usize;
            type Output = usize;
            type Error = ();
            fn run<'a>(
                &'a self,
                _rt: &'a TestRuntime,
                input: usize,
            ) -> LocalBoxFuture<'a, Result<usize, ()>> {
                Box::pin(async move { Ok(input * 2) })
            }
        }

        let step = Step::task(|_rt: &TestRuntime, input: usize| {
            Box::pin(async move { Ok::<usize, ()>(input + 1) })
        });

        let pipeline = Pipeline::builder()
            .add_step(PhaseId::new("step1"), step)
            .add_step(PhaseId::new("step2"), Step::builder(Doubler).build())
            .with_route(PhaseId::new("step1"), Route::next("step2"))
            .with_route(PhaseId::new("step2"), Route::Halt)
            .with_initial(PhaseId::new("step1"))
            .build()
            .unwrap();

        let result: usize = pipeline.run(&TestRuntime, 3usize).await.unwrap();
        assert_eq!(result, 8); // (3 + 1) * 2
    }

    #[test]
    fn arc_downcast_works() {
        let arc: Arc<dyn Any + Send + Sync> = Arc::new(42usize);
        let downcast = Arc::downcast::<usize>(arc).unwrap();
        assert_eq!(*downcast, 42);
    }

    #[test]
    fn arc_downcast_with_as_cast() {
        let arc = Arc::new(42usize) as Arc<dyn Any + Send + Sync>;
        let downcast = Arc::downcast::<usize>(arc).unwrap();
        assert_eq!(*downcast, 42);
    }

    #[test]
    fn arc_downcast_in_closure() {
        let runner = |input: Arc<dyn Any + Send + Sync>| Arc::downcast::<usize>(input).unwrap();
        let result = runner(Arc::new(42usize) as Arc<dyn Any + Send + Sync>);
        assert_eq!(*result, 42);
    }

    #[tokio::test]
    async fn mimicked_pipeline_runner() {
        type Runner = Arc<dyn Fn(Arc<dyn Any + Send + Sync>) -> Arc<dyn Any + Send + Sync>>;
        let runner: Runner = Arc::new(move |input| {
            let typed = Arc::downcast::<usize>(input).unwrap();
            Arc::new((*typed) + 1) as Arc<dyn Any + Send + Sync>
        });
        let input = Arc::new(3usize) as Arc<dyn Any + Send + Sync>;
        let output = runner(input);
        let result = Arc::downcast::<usize>(output).unwrap();
        assert_eq!(*result, 4);
    }

    #[tokio::test]
    async fn minimal_pipeline() {
        #[derive(Clone)]
        struct Inc;
        impl Phase for Inc {
            type Runtime = TestRuntime;
            type Input = usize;
            type Output = usize;
            type Error = ();
            fn run<'a>(
                &'a self,
                _rt: &'a TestRuntime,
                input: usize,
            ) -> LocalBoxFuture<'a, Result<usize, ()>> {
                Box::pin(async move { Ok(input + 1) })
            }
        }

        let pipeline = Pipeline::builder()
            .add_phase(PhaseId::new("inc"), Inc)
            .with_route(PhaseId::new("inc"), Route::Halt)
            .with_initial(PhaseId::new("inc"))
            .build()
            .unwrap();

        let result: usize = pipeline.run(&TestRuntime, 5usize).await.unwrap();
        assert_eq!(result, 6);
    }

    // 1.7 Validation: type mismatch between phases
    #[test]
    fn validation_catches_type_mismatch() {
        #[derive(Clone)]
        struct A;
        impl Phase for A {
            type Runtime = TestRuntime;
            type Input = usize;
            type Output = usize;
            type Error = ();
            fn run<'a>(
                &'a self,
                _rt: &'a TestRuntime,
                input: usize,
            ) -> LocalBoxFuture<'a, Result<usize, ()>> {
                Box::pin(async move { Ok(input) })
            }
        }

        #[derive(Clone)]
        struct B;
        impl Phase for B {
            type Runtime = TestRuntime;
            type Input = String;
            type Output = String;
            type Error = ();
            fn run<'a>(
                &'a self,
                _rt: &'a TestRuntime,
                input: String,
            ) -> LocalBoxFuture<'a, Result<String, ()>> {
                Box::pin(async move { Ok(input) })
            }
        }

        let result = Pipeline::builder()
            .add_phase(PhaseId::new("a"), A)
            .add_phase(PhaseId::new("b"), B)
            .with_route(PhaseId::new("a"), Route::next("b"))
            .with_initial(PhaseId::new("a"))
            .build();

        assert!(result.is_err());
    }
}
