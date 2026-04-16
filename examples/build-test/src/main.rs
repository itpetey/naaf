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
//!                                     (accepted)          RepairPatch ──► FeatureRequest
//!                                                             (loops back to GeneratePatch)
//! ```

use std::fmt::{Display, Formatter};

use futures::future::LocalBoxFuture;
use naaf_core::{Attempt, Check, Materialiser, RepairPlanner, RetryPolicy, Step, Task};

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

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("build error")
    }
}

impl std::error::Error for Error {}

struct GeneratePatch;

impl Task for GeneratePatch {
    type Runtime = BuildRuntime;
    type Input = FeatureRequest;
    type Output = Patch;
    type Error = Error;

    fn run<'a>(
        &'a self,
        _runtime: &'a Self::Runtime,
        input: Self::Input,
    ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
        Box::pin(async move {
            let files = vec![
                format!("src/{name}/mod.rs", name = input.name),
                format!("src/{name}/tests.rs", name = input.name),
            ];
            Ok(Patch {
                revision: input.revision,
                files,
            })
        })
    }
}

struct ApplyPatch;

impl Materialiser for ApplyPatch {
    type Runtime = BuildRuntime;
    type Input = Patch;
    type Output = Workspace;
    type Error = Error;

    fn materialise<'a>(
        &'a self,
        _runtime: &'a Self::Runtime,
        input: Self::Input,
    ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
        Box::pin(async move {
            Ok(Workspace {
                revision: input.revision,
            })
        })
    }
}

struct CargoTest;

impl Check for CargoTest {
    type Runtime = BuildRuntime;
    type Subject = Workspace;
    type Finding = TestFinding;
    type Error = Error;

    fn check<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        subject: Self::Subject,
    ) -> LocalBoxFuture<'a, Result<Vec<Self::Finding>, Self::Error>> {
        Box::pin(async move {
            if subject.revision >= runtime.required_revision {
                Ok(Vec::new())
            } else {
                let failed = runtime.required_revision - subject.revision;
                Ok(vec![TestFinding::TestsFailed {
                    passed: subject.revision,
                    failed,
                }])
            }
        })
    }
}

struct RepairPatch;

impl RepairPlanner for RepairPatch {
    type Runtime = BuildRuntime;
    type Input = FeatureRequest;
    type Artefact = Patch;
    type Finding = TestFinding;
    type Error = Error;

    fn repair<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        attempts: Vec<Attempt<Self::Input, Self::Artefact, Self::Finding>>,
    ) -> LocalBoxFuture<'a, Result<Self::Input, Self::Error>> {
        Box::pin(async move {
            let previous = attempts.last().expect("attempt present");
            Ok(FeatureRequest {
                name: previous.input.name.clone(),
                revision: previous.artefact.revision + runtime.repair_increment,
            })
        })
    }
}

#[tokio::main]
async fn main() {
    let runtime = BuildRuntime {
        required_revision: 3,
        repair_increment: 2,
    };

    let build_step = Step::builder(GeneratePatch)
        .materialise(ApplyPatch)
        .validate(CargoTest)
        .repair_with(RepairPatch)
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

    println!("Final patch revision: {}", traced.output().revision);
    println!("Files: {:?}", traced.output().files);
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
