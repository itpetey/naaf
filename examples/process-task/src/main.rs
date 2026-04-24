//! Demonstrates `ProcessTask` from `naaf-process`, which adapts shell commands
//! into `naaf_core::Task`.
//!
//! A `ProcessTask` runs a `printf` command to produce output, a closure-based
//! check validates the result, and a repair planner adjusts the command. The
//! step retries until the check passes.

use std::{convert::Infallible, string::FromUtf8Error};

use naaf_core::{Attempt, RetryPolicy, Step, check_fn, repair_fn};
use naaf_process::{AdaptorError, ProcessAgent, ProcessCommand};

#[derive(Debug)]
struct CheckRuntime {
    required_substring: &'static str,
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

    let mentions_substring = check_fn(|runtime: &CheckRuntime, _input: String, subject: String| {
        let findings: Vec<&'static str> = if subject.contains(runtime.required_substring) {
            Vec::new()
        } else {
            vec!["output did not contain the required substring"]
        };
        Box::pin(async move { Ok::<_, AdaptorError<Infallible, FromUtf8Error>>(findings) })
    });

    let revise_command = repair_fn(
        |runtime: &CheckRuntime, _attempts: Vec<Attempt<String, String, &'static str>>| {
            Box::pin(async move {
                Ok::<_, AdaptorError<Infallible, FromUtf8Error>>(format!(
                    "printf '{}'",
                    runtime.required_substring
                ))
            })
        },
    );

    let step = Step::builder(echo_task)
        .validate(mentions_substring)
        .repair_with(revise_command)
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
