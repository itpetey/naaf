//! Local process-backed `naaf_core` role infrastructure.
//!
//! `naaf-process` adapts shell commands and direct process invocations into the
//! `naaf_core` workflow contracts. Callers stay in control of how commands are
//! built and how successful output is decoded.
//!
//! A shared [`ProcessAgent`] can then be projected into typed `Task`, `Check`,
//! `Materialiser`, and `RepairPlanner` implementations without duplicating the
//! command execution wiring.
//!
//! # Example
//!
//! ```
//! use std::convert::Infallible;
//!
//! use naaf_core::{Check, Step, Task};
//! use naaf_process::{CheckError, ProcessAgent, ProcessCommand, ProcessOutput};
//!
//! #[derive(Debug, Default)]
//! struct Runtime;
//!
//! struct MentionsHello;
//!
//! impl Check for MentionsHello {
//!     type Runtime = Runtime;
//!     type Subject = String;
//!     type Finding = &'static str;
//!     type Error = CheckError<Infallible, std::string::FromUtf8Error>;
//!
//!     fn check<'a>(
//!         &'a self,
//!         _runtime: &'a Self::Runtime,
//!         subject: Self::Subject,
//!     ) -> futures::future::LocalBoxFuture<'a, Result<Vec<Self::Finding>, Self::Error>> {
//!         Box::pin(async move {
//!             if subject.contains("hello") {
//!                 Ok(Vec::new())
//!             } else {
//!                 Ok(vec!["stdout did not contain the expected text"])
//!             }
//!         })
//!     }
//! }
//!
//! let process = ProcessAgent::new();
//! let task = process.task(
//!     |_runtime: &Runtime, script: String| Ok::<_, Infallible>(ProcessCommand::shell(script)),
//!     |output: ProcessOutput| String::from_utf8(output.stdout),
//! );
//!
//! tokio::runtime::Runtime::new()
//!     .expect("runtime should build")
//!     .block_on(async {
//!         let step = Step::builder(task).validate(MentionsHello).build();
//!         let traced = step
//!             .run_traced(&Runtime, "printf 'hello from process task'".to_string())
//!             .await
//!             .expect("step should succeed");
//!
//!         assert_eq!(traced.output(), "hello from process task");
//!         assert!(traced.report().attempts()[0].accepted());
//!     });
//! ```

pub use crate::{
    agent::ProcessAgent,
    command::{ProcessCommand, ProcessOutput},
    error::{
        AdaptorError, CheckError, MaterialiserError, ProcessError, RepairPlannerError, TaskError,
    },
    task::{ProcessCheck, ProcessMaterialiser, ProcessRepairPlanner, ProcessTask},
};

mod agent;
mod command;
mod error;
mod task;
