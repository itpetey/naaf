//! Demonstrates `ProcessTask` from `naaf-process`, which adapts shell commands
//! into `naaf_core::Task`.
//!
//! A `ProcessTask` runs a `printf` command to produce output, a hand-written
//! check validates the result, and a repair planner adjusts the command. The
//! step retries until the check passes.

use std::{convert::Infallible, string::FromUtf8Error};

use futures::future::LocalBoxFuture;
use naaf_core::{Attempt, Check, RepairPlanner, RetryPolicy, Step};
use naaf_process::{AdapterError, ProcessAgent, ProcessCommand};

#[derive(Debug)]
struct CheckRuntime {
    required_substring: &'static str,
}

struct MentionsSubstring;

impl Check for MentionsSubstring {
    type Runtime = CheckRuntime;
    type Subject = String;
    type Finding = &'static str;
    type Error = AdapterError<Infallible, FromUtf8Error>;

    fn check<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        subject: Self::Subject,
    ) -> LocalBoxFuture<'a, Result<Vec<Self::Finding>, Self::Error>> {
        Box::pin(async move {
            if subject.contains(runtime.required_substring) {
                Ok(Vec::new())
            } else {
                Ok(vec!["output did not contain the required substring"])
            }
        })
    }
}

struct ReviseCommand;

impl RepairPlanner for ReviseCommand {
    type Runtime = CheckRuntime;
    type Input = String;
    type Artefact = String;
    type Finding = &'static str;
    type Error = AdapterError<Infallible, FromUtf8Error>;

    fn repair<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        _attempts: Vec<Attempt<Self::Input, Self::Artefact, Self::Finding>>,
    ) -> LocalBoxFuture<'a, Result<Self::Input, Self::Error>> {
        Box::pin(async move { Ok(format!("printf '{}'", runtime.required_substring)) })
    }
}

#[tokio::main]
async fn main() {
    let agent = ProcessAgent::new();

    let echo_task = agent.task(
        |_runtime: &CheckRuntime, command: String| {
            Ok::<_, Infallible>(ProcessCommand::shell(command))
        },
        |output: naaf_process::ProcessOutput| String::from_utf8(output.stdout),
    );

    let runtime = CheckRuntime {
        required_substring: "hello from naaf",
    };

    let step = Step::builder(echo_task)
        .validate(MentionsSubstring)
        .repair_with(ReviseCommand)
        .retry_policy(RetryPolicy::new(3))
        .build();

    let traced = step
        .run_traced(&runtime, "printf 'wrong output'".to_string())
        .await
        .expect("step should repair and succeed");

    println!("Output: {}", traced.output().trim());
    println!("Attempts: {}", traced.report().attempt_count());
    for (i, attempt) in traced.report().attempts().iter().enumerate() {
        println!("  Attempt {}: accepted={}", i + 1, attempt.accepted);
    }
}
