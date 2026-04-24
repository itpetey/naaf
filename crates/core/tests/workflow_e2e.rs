use std::sync::{Arc, Mutex};

use futures::future::LocalBoxFuture;
use naaf_core::{Attempt, Check, Materialiser, RepairPlanner, RetryPolicy, Step, Task};

struct ReconcileRelease;

struct RepairPatch;

struct GenerateApi;

struct GenerateUi;

struct PublishRelease;

struct GeneratePatch;

struct ApplyPatch;

struct CargoTest;

#[derive(Debug)]
struct TestRuntime {
    required_revision: usize,
    repair_increment: usize,
    release_suffix: &'static str,
    events: Arc<Mutex<Vec<&'static str>>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TestError;

#[derive(Clone, Debug, PartialEq, Eq)]
struct FeatureRequest {
    name: &'static str,
    revision: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ApiDraft {
    files: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct UiDraft {
    files: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ReleasePlan {
    feature: String,
    files: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ReleaseSummary {
    tag: String,
    file_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BuildInput {
    revision: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Patch {
    revision: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Workspace {
    revision: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Finding {
    TestsFailed,
}

impl TestRuntime {
    fn new(
        required_revision: usize,
        repair_increment: usize,
        release_suffix: &'static str,
    ) -> Self {
        Self {
            required_revision,
            repair_increment,
            release_suffix,
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn record(&self, event: &'static str) {
        self.events.lock().expect("events lock").push(event);
    }

    fn occurrences(&self, event: &str) -> usize {
        self.events
            .lock()
            .expect("events lock")
            .iter()
            .filter(|entry| **entry == event)
            .count()
    }
}

impl std::error::Error for TestError {}

impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("test error")
    }
}

impl Task for GenerateApi {
    type Runtime = TestRuntime;
    type Input = FeatureRequest;
    type Output = ApiDraft;
    type Error = TestError;

    fn run<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        input: Self::Input,
    ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
        Box::pin(async move {
            runtime.record("generate_api");
            Ok(ApiDraft {
                files: vec![format!("src/{}/api.rs", input.name)],
            })
        })
    }
}

impl Task for GenerateUi {
    type Runtime = TestRuntime;
    type Input = FeatureRequest;
    type Output = UiDraft;
    type Error = TestError;

    fn run<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        input: Self::Input,
    ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
        Box::pin(async move {
            runtime.record("generate_ui");
            Ok(UiDraft {
                files: vec![format!("src/{}/ui.rs", input.name)],
            })
        })
    }
}

impl Task for ReconcileRelease {
    type Runtime = TestRuntime;
    type Input = (ApiDraft, UiDraft);
    type Output = ReleasePlan;
    type Error = TestError;

    fn run<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        input: Self::Input,
    ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
        Box::pin(async move {
            runtime.record("reconcile_release");
            let mut files = input.0.files;
            files.extend(input.1.files);
            files.sort();

            Ok(ReleasePlan {
                feature: format!("release-{}", runtime.release_suffix),
                files,
            })
        })
    }
}

impl Task for PublishRelease {
    type Runtime = TestRuntime;
    type Input = ReleasePlan;
    type Output = ReleaseSummary;
    type Error = TestError;

    fn run<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        input: Self::Input,
    ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
        Box::pin(async move {
            runtime.record("publish_release");
            Ok(ReleaseSummary {
                tag: format!("{}-{}", input.feature, runtime.release_suffix),
                file_count: input.files.len(),
            })
        })
    }
}

impl Task for GeneratePatch {
    type Runtime = TestRuntime;
    type Input = BuildInput;
    type Output = Patch;
    type Error = TestError;

    fn run<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        input: Self::Input,
    ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
        Box::pin(async move {
            runtime.record("generate_patch");
            Ok(Patch {
                revision: input.revision,
            })
        })
    }
}

impl Materialiser for ApplyPatch {
    type Runtime = TestRuntime;
    type Input = Patch;
    type Output = Workspace;
    type Error = TestError;

    fn materialise<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        input: Self::Input,
    ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
        Box::pin(async move {
            runtime.record("apply_patch");
            Ok(Workspace {
                revision: input.revision,
            })
        })
    }
}

impl Check for CargoTest {
    type Runtime = TestRuntime;
    type Input = BuildInput;
    type Output = Workspace;
    type Finding = Finding;
    type Error = TestError;

    fn check<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        _input: Self::Input,
        output: Self::Output,
    ) -> LocalBoxFuture<'a, Result<Vec<Self::Finding>, Self::Error>> {
        Box::pin(async move {
            runtime.record("cargo_test");
            if output.revision >= runtime.required_revision {
                Ok(Vec::new())
            } else {
                Ok(vec![Finding::TestsFailed])
            }
        })
    }
}

impl RepairPlanner for RepairPatch {
    type Runtime = TestRuntime;
    type Input = BuildInput;
    type Output = Patch;
    type Finding = Finding;
    type Error = TestError;

    fn repair<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        attempts: Vec<Attempt<Self::Input, Self::Output, Self::Finding>>,
    ) -> LocalBoxFuture<'a, Result<Self::Input, Self::Error>> {
        Box::pin(async move {
            runtime.record("repair_patch");
            let previous = attempts.last().expect("attempt present");
            Ok(BuildInput {
                revision: previous.output.revision + runtime.repair_increment,
            })
        })
    }
}

#[tokio::test]
async fn step_build_test_loop_retries_through_public_api() {
    let runtime = TestRuntime::new(2, 1, "candidate");

    let step = Step::builder(GeneratePatch)
        .materialise(ApplyPatch)
        .validate(CargoTest)
        .repair_with(RepairPatch)
        .retry_policy(RetryPolicy::new(3))
        .build();

    let traced = step
        .run_traced(&runtime, BuildInput { revision: 0 })
        .await
        .expect("step should repair itself");

    assert_eq!(traced.output(), &Patch { revision: 2 });
    assert_eq!(traced.report().attempt_count(), 3);
    assert_eq!(
        traced
            .report()
            .attempts()
            .iter()
            .filter(|attempt| attempt.accepted())
            .count(),
        1
    );
    assert_eq!(runtime.occurrences("generate_patch"), 3);
    assert_eq!(runtime.occurrences("apply_patch"), 3);
    assert_eq!(runtime.occurrences("cargo_test"), 3);
    assert_eq!(runtime.occurrences("repair_patch"), 2);
}

#[tokio::test]
async fn workflow_composition_transports_outputs_end_to_end() {
    let runtime = TestRuntime::new(0, 1, "candidate");

    let workflow = Step::builder(GenerateApi)
        .with_findings::<Finding>()
        .build()
        .join(Step::builder(GenerateUi).with_findings::<Finding>().build())
        .reconcile_task(ReconcileRelease)
        .then(
            Step::builder(PublishRelease)
                .with_findings::<Finding>()
                .build(),
        );

    let traced = workflow
        .run_traced(
            &runtime,
            FeatureRequest {
                name: "search",
                revision: 0,
            },
        )
        .await
        .expect("workflow should succeed");

    assert_eq!(
        traced.output(),
        &ReleaseSummary {
            tag: "release-candidate-candidate".to_string(),
            file_count: 2,
        }
    );
    assert_eq!(traced.report().attempt_count(), 4);
    assert!(
        traced
            .report()
            .attempts()
            .iter()
            .all(|attempt| attempt.accepted())
    );
    assert!(
        traced
            .report()
            .attempts()
            .iter()
            .all(|attempt| attempt.findings.is_empty())
    );
    assert_eq!(runtime.occurrences("generate_api"), 1);
    assert_eq!(runtime.occurrences("generate_ui"), 1);
    assert_eq!(runtime.occurrences("reconcile_release"), 1);
    assert_eq!(runtime.occurrences("publish_release"), 1);
}
