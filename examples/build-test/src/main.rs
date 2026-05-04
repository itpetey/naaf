//! Demonstrates a feature-implementation workflow that generates a patch,
//! materialises it into a workspace, and validates with a test suite.
//!
//! The step retries automatically: when tests fail, the repair planner bumps
//! the revision so a new patch is generated, applied, and tested again.
//!
//! This is the core generate → materialise → validate → repair loop:
//!
//! ```text
//! FeatureRequest ──► GeneratePatch ──► Patch
//!                                         │
//!                                    ApplyPatch
//!                                         │
//!                                         ▼
//!                                    Workspace ──► CargoTest ──► findings
//!                                         │                        │
//!                                         │           (if non-empty)
//!                                         │                        ▼
//!                                (accepted output)        RepairPatch ──► FeatureRequest
//!                                                             (loops back to GeneratePatch)
//! ```

use std::fmt::{Display, Formatter};

use naaf_core::{Attempt, RetryPolicy, Step, check_fn, materialiser_fn, repair_last_fn, task_fn};

#[derive(Debug)]
struct BuildRuntime {
    required_revision: usize,
    repair_increment: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FeatureRequest {
    name: String,
    revision: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Patch {
    revision: usize,
    files: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Workspace {
    revision: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum TestFinding {
    TestsFailed { passed: usize, failed: usize },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Error;

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("build error")
    }
}

#[tokio::main]
async fn main() {
    let runtime = BuildRuntime {
        required_revision: 3,
        repair_increment: 2,
    };

    let generate_patch = task_fn(|_runtime: &BuildRuntime, input: FeatureRequest| {
        let files = vec![
            format!("src/{name}/mod.rs", name = input.name),
            format!("src/{name}/tests.rs", name = input.name),
        ];
        Box::pin(async move {
            Ok::<_, Error>(Patch {
                revision: input.revision,
                files,
            })
        })
    });

    let apply_patch = materialiser_fn(|_runtime: &BuildRuntime, input: Patch| {
        Box::pin(async move {
            Ok::<_, Error>(Workspace {
                revision: input.revision,
            })
        })
    });

    let cargo_test = check_fn(
        |runtime: &BuildRuntime, _input: FeatureRequest, subject: Workspace| {
            let findings = if subject.revision >= runtime.required_revision {
                Vec::new()
            } else {
                let failed = runtime.required_revision - subject.revision;
                vec![TestFinding::TestsFailed {
                    passed: subject.revision,
                    failed,
                }]
            };
            Box::pin(async move { Ok::<_, Error>(findings) })
        },
    );

    let repair_patch = repair_last_fn(
        |runtime: &BuildRuntime, last: Attempt<FeatureRequest, Patch, TestFinding>| {
            Box::pin(async move {
                Ok::<_, Error>(FeatureRequest {
                    name: last.input.name,
                    revision: last.output.revision + runtime.repair_increment,
                })
            })
        },
    );

    let build_step = Step::builder(generate_patch)
        .materialise(apply_patch)
        .validate(cargo_test)
        .repair_with(repair_patch)
        .retry_policy(RetryPolicy::new(5))
        .build();

    let request = FeatureRequest {
        name: "search".to_string(),
        revision: 0,
    };

    let traced = build_step
        .run_traced(&runtime, request)
        .await
        .expect("build step should succeed after repair");

    println!("Final workspace revision: {}", traced.output().revision);
    println!("Attempts: {}", traced.report().attempt_count());
    for (i, attempt) in traced.report().attempts().iter().enumerate() {
        match &attempt.findings[..] {
            [] => println!("  Attempt {}: accepted", i + 1),
            findings => println!(
                "  Attempt {}: {} finding(s), rejected",
                i + 1,
                findings.len()
            ),
        }
    }
}
