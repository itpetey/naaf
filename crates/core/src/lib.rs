//! `naaf` is a strongly typed orchestration library for building asynchronous
//! LLM workflows out of local retrying `Step`s and declarative `Pipeline`s.
//!
//! A `Step` owns one task plus its nearby checks, optional materialisation, and
//! optional repair planning. `Pipeline` composes typed `Phase`s with declared
//! routes (`Next`, `Switch`, `Parallel`, `Halt`), supporting cycles for
//! backflow and conditional branching.
//!
//! The runtime is threaded explicitly through every trait, which keeps domain
//! types clean while allowing checks and materialisers to use side effects such
//! as filesystem access, command execution, or model routing.
//!
//! # End-To-End Step
//!
//! ```
//! use futures::future::LocalBoxFuture;
//! use naaf_core::{Attempt, Check, Materialiser, RepairPlanner, RetryPolicy, Step, Task};
//!
//! #[derive(Debug)]
//! struct Runtime {
//!     required_revision: usize,
//! }
//!
//! #[derive(Clone, Debug, PartialEq, Eq)]
//! struct Prompt {
//!     revision: usize,
//! }
//!
//! #[derive(Clone, Debug, PartialEq, Eq)]
//! struct Patch {
//!     revision: usize,
//! }
//!
//! #[derive(Clone, Debug, PartialEq, Eq)]
//! struct Worktree {
//!     revision: usize,
//! }
//!
//! #[derive(Clone, Debug, PartialEq, Eq)]
//! enum Finding {
//!     TestsFailed,
//! }
//!
//! #[derive(Clone, Copy, Debug, PartialEq, Eq)]
//! struct Error;
//!
//! impl std::fmt::Display for Error {
//!     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//!         f.write_str("error")
//!     }
//! }
//!
//! impl std::error::Error for Error {}
//!
//! struct Generate;
//!
//! impl Task for Generate {
//!     type Runtime = Runtime;
//!     type Input = Prompt;
//!     type Output = Patch;
//!     type Error = Error;
//!
//!     fn run<'a>(
//!         &'a self,
//!         _runtime: &'a Self::Runtime,
//!         input: Self::Input,
//!     ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
//!         Box::pin(async move { Ok(Patch { revision: input.revision }) })
//!     }
//! }
//!
//! struct ApplyPatch;
//!
//! impl Materialiser for ApplyPatch {
//!     type Runtime = Runtime;
//!     type Input = Patch;
//!     type Output = Worktree;
//!     type Error = Error;
//!
//!     fn materialise<'a>(
//!         &'a self,
//!         _runtime: &'a Self::Runtime,
//!         input: Self::Input,
//!     ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
//!         Box::pin(async move { Ok(Worktree { revision: input.revision }) })
//!     }
//! }
//!
//! struct CargoTest;
//!
//! impl Check for CargoTest {
//!     type Runtime = Runtime;
//!     type Input = Prompt;
//!     type Output = Worktree;
//!     type Finding = Finding;
//!     type Error = Error;
//!
//!     fn check<'a>(
//!         &'a self,
//!         runtime: &'a Self::Runtime,
//!         _input: Self::Input,
//!         output: Self::Output,
//!     ) -> LocalBoxFuture<'a, Result<Vec<Self::Finding>, Self::Error>> {
//!         Box::pin(async move {
//!             if output.revision >= runtime.required_revision {
//!                 Ok(Vec::new())
//!             } else {
//!                 Ok(vec![Finding::TestsFailed])
//!             }
//!         })
//!     }
//! }
//!
//! struct Repair;
//!
//! impl RepairPlanner for Repair {
//!     type Runtime = Runtime;
//!     type Input = Prompt;
//!     type Output = Patch;
//!     type Finding = Finding;
//!     type Error = Error;
//!
//!     fn repair<'a>(
//!         &'a self,
//!         _runtime: &'a Self::Runtime,
//!         attempts: Vec<Attempt<Self::Input, Self::Output, Self::Finding>>,
//!     ) -> LocalBoxFuture<'a, Result<Self::Input, Self::Error>> {
//!         Box::pin(async move {
//!             let previous = attempts.last().expect("attempt present");
//!             Ok(Prompt {
//!                 revision: previous.output.revision + 1,
//!             })
//!         })
//!     }
//! }
//!
//! tokio::runtime::Runtime::new()
//!     .expect("runtime should build")
//!     .block_on(async {
//!     let runtime = Runtime {
//!         required_revision: 2,
//!     };
//!
//!     let step = Step::builder(Generate)
//!         .materialise(ApplyPatch)
//!         .validate(CargoTest)
//!         .repair_with(Repair)
//!         .retry_policy(RetryPolicy::new(3))
//!         .build();
//!
//!     let traced = step
//!         .run_traced(&runtime, Prompt { revision: 0 })
//!         .await
//!         .expect("step should recover");
//!
//!     assert_eq!(traced.output().revision, 2);
//!     assert_eq!(traced.report().attempt_count(), 3);
//!     assert!(traced.report().attempts()[2].accepted());
//!     });
//! ```
//!
pub use crate::{
    adaptor::{check_fn, materialiser_fn, repair_fn, repair_last_fn, task_fn},
    check::Check,
    checkpoint::{
        CheckpointResult, PipelineCheckpoint, PipelineCheckpointer, StepCheckpoint,
        StepCheckpointer,
    },
    materialiser::Materialiser,
    observability::{
        CheckExt, MaterialiserExt, ObservedCheck, ObservedMaterialiser, ObservedRepairPlanner,
        ObservedTask, RepairPlannerExt, TaskExt,
    },
    pipeline::{
        Phase, PhaseId, Pipeline, PipelineBuilder, PipelineError, PipelineValidationError, Route,
    },
    repair::{Attempt, AttemptReport, RepairPlanner, RetryPolicy, StepReport, Traced},
    step::{BoundStepBuilder, OpenStepBuilder, Step, StepBuilder, StepError, SystemStage},
    task::Task,
};

mod adaptor;
mod check;
mod checkpoint;
mod materialiser;
mod observability;
mod pipeline;
mod repair;
pub mod span;
mod step;
mod task;
