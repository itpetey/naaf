use std::{any::type_name, borrow::Cow, fmt::Debug};

use futures::future::LocalBoxFuture;
use tracing::{Instrument, debug, debug_span, error, trace};

use crate::{Attempt, Check, Materialiser, RepairPlanner, Task};

/// Extension trait for wrapping tasks with structured `tracing` events.
pub trait TaskExt: Task + Sized {
    /// Wraps the task and emits lifecycle events, inputs, and outputs.
    fn observed(self) -> ObservedTask<Self>
    where
        Self::Input: Debug,
        Self::Output: Debug,
        Self::Error: Debug,
    {
        ObservedTask::new(self, type_name::<Self>())
    }

    /// Wraps the task and uses a custom component name in emitted events.
    fn observed_as(self, name: impl Into<Cow<'static, str>>) -> ObservedTask<Self>
    where
        Self::Input: Debug,
        Self::Output: Debug,
        Self::Error: Debug,
    {
        ObservedTask::new(self, name)
    }
}

impl<T> TaskExt for T where T: Task {}

/// Extension trait for wrapping checks with structured `tracing` events.
pub trait CheckExt: Check + Sized {
    /// Wraps the check and emits lifecycle events, inputs, and findings.
    fn observed(self) -> ObservedCheck<Self>
    where
        Self::Subject: Debug,
        Self::Finding: Debug,
        Self::Error: Debug,
    {
        ObservedCheck::new(self, type_name::<Self>())
    }

    /// Wraps the check and uses a custom component name in emitted events.
    fn observed_as(self, name: impl Into<Cow<'static, str>>) -> ObservedCheck<Self>
    where
        Self::Subject: Debug,
        Self::Finding: Debug,
        Self::Error: Debug,
    {
        ObservedCheck::new(self, name)
    }
}

impl<T> CheckExt for T where T: Check {}

/// Extension trait for wrapping materialisers with structured `tracing` events.
pub trait MaterialiserExt: Materialiser + Sized {
    /// Wraps the materialiser and emits lifecycle events, inputs, and outputs.
    fn observed(self) -> ObservedMaterialiser<Self>
    where
        Self::Input: Debug,
        Self::Output: Debug,
        Self::Error: Debug,
    {
        ObservedMaterialiser::new(self, type_name::<Self>())
    }

    /// Wraps the materialiser and uses a custom component name in emitted events.
    fn observed_as(self, name: impl Into<Cow<'static, str>>) -> ObservedMaterialiser<Self>
    where
        Self::Input: Debug,
        Self::Output: Debug,
        Self::Error: Debug,
    {
        ObservedMaterialiser::new(self, name)
    }
}

impl<T> MaterialiserExt for T where T: Materialiser {}

/// Extension trait for wrapping repair planners with structured `tracing` events.
pub trait RepairPlannerExt: RepairPlanner + Sized {
    /// Wraps the repair planner and emits lifecycle events, attempts, and next input.
    fn observed(self) -> ObservedRepairPlanner<Self>
    where
        Self::Input: Debug,
        Self::Artefact: Debug,
        Self::Finding: Debug,
        Self::Error: Debug,
    {
        ObservedRepairPlanner::new(self, type_name::<Self>())
    }

    /// Wraps the repair planner and uses a custom component name in emitted events.
    fn observed_as(self, name: impl Into<Cow<'static, str>>) -> ObservedRepairPlanner<Self>
    where
        Self::Input: Debug,
        Self::Artefact: Debug,
        Self::Finding: Debug,
        Self::Error: Debug,
    {
        ObservedRepairPlanner::new(self, name)
    }
}

impl<T> RepairPlannerExt for T where T: RepairPlanner {}

/// A task wrapper that emits structured `tracing` events around execution.
pub struct ObservedTask<T> {
    inner: T,
    name: Cow<'static, str>,
}

impl<T> ObservedTask<T> {
    fn new(inner: T, name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            inner,
            name: name.into(),
        }
    }
}

impl<T> Task for ObservedTask<T>
where
    T: Task,
    T::Input: Debug,
    T::Output: Debug,
    T::Error: Debug,
{
    type Runtime = T::Runtime;
    type Input = T::Input;
    type Output = T::Output;
    type Error = T::Error;

    fn run<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        input: Self::Input,
    ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
        let task = self.name.clone();
        let inner = &self.inner;
        let span = debug_span!(
            "task_run",
            component = "task",
            task = %task,
            input_type = %type_name::<T::Input>(),
            output_type = %type_name::<T::Output>()
        );

        Box::pin(
            async move {
                trace!(action = "input", input = ?input, "task input");
                debug!(action = "run.start", "task started");

                match inner.run(runtime, input).await {
                    Ok(output) => {
                        trace!(action = "output", output = ?output, "task output");
                        debug!(action = "run.complete", "task completed");
                        Ok(output)
                    }
                    Err(error_value) => {
                        trace!(action = "error", error = ?error_value, "task error");
                        error!(action = "run.error", "task failed");
                        Err(error_value)
                    }
                }
            }
            .instrument(span),
        )
    }
}

/// A check wrapper that emits structured `tracing` events around execution.
pub struct ObservedCheck<C> {
    inner: C,
    name: Cow<'static, str>,
}

impl<C> ObservedCheck<C> {
    fn new(inner: C, name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            inner,
            name: name.into(),
        }
    }
}

impl<C> Check for ObservedCheck<C>
where
    C: Check,
    C::Subject: Debug,
    C::Finding: Debug,
    C::Error: Debug,
{
    type Runtime = C::Runtime;
    type Subject = C::Subject;
    type Finding = C::Finding;
    type Error = C::Error;

    fn check<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        subject: Self::Subject,
    ) -> LocalBoxFuture<'a, Result<Vec<Self::Finding>, Self::Error>> {
        let check = self.name.clone();
        let inner = &self.inner;
        let span = debug_span!(
            "check_run",
            component = "check",
            check = %check,
            subject_type = %type_name::<C::Subject>(),
            finding_type = %type_name::<C::Finding>()
        );

        Box::pin(
            async move {
                trace!(action = "input", subject = ?subject, "check subject");
                debug!(action = "run.start", "check started");

                match inner.check(runtime, subject).await {
                    Ok(findings) => {
                        let finding_count = findings.len();
                        trace!(action = "output", findings = ?findings, "check findings");
                        debug!(action = "run.complete", finding_count, "check completed");
                        Ok(findings)
                    }
                    Err(error_value) => {
                        trace!(action = "error", error = ?error_value, "check error");
                        error!(action = "run.error", "check failed");
                        Err(error_value)
                    }
                }
            }
            .instrument(span),
        )
    }
}

/// A materialiser wrapper that emits structured `tracing` events around execution.
pub struct ObservedMaterialiser<M> {
    inner: M,
    name: Cow<'static, str>,
}

impl<M> ObservedMaterialiser<M> {
    fn new(inner: M, name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            inner,
            name: name.into(),
        }
    }
}

impl<M> Materialiser for ObservedMaterialiser<M>
where
    M: Materialiser,
    M::Input: Debug,
    M::Output: Debug,
    M::Error: Debug,
{
    type Runtime = M::Runtime;
    type Input = M::Input;
    type Output = M::Output;
    type Error = M::Error;

    fn materialise<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        input: Self::Input,
    ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
        let materialiser = self.name.clone();
        let inner = &self.inner;
        let span = debug_span!(
            "materialiser_run",
            component = "materialiser",
            materialiser = %materialiser,
            input_type = %type_name::<M::Input>(),
            output_type = %type_name::<M::Output>()
        );

        Box::pin(
            async move {
                trace!(action = "input", input = ?input, "materialiser input");
                debug!(action = "run.start", "materialiser started");

                match inner.materialise(runtime, input).await {
                    Ok(output) => {
                        trace!(action = "output", output = ?output, "materialiser output");
                        debug!(action = "run.complete", "materialiser completed");
                        Ok(output)
                    }
                    Err(error_value) => {
                        trace!(action = "error", error = ?error_value, "materialiser error");
                        error!(action = "run.error", "materialiser failed");
                        Err(error_value)
                    }
                }
            }
            .instrument(span),
        )
    }
}

/// A repair planner wrapper that emits structured `tracing` events around execution.
pub struct ObservedRepairPlanner<P> {
    inner: P,
    name: Cow<'static, str>,
}

impl<P> ObservedRepairPlanner<P> {
    fn new(inner: P, name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            inner,
            name: name.into(),
        }
    }
}

impl<P> RepairPlanner for ObservedRepairPlanner<P>
where
    P: RepairPlanner,
    P::Input: Debug,
    P::Artefact: Debug,
    P::Finding: Debug,
    P::Error: Debug,
{
    type Runtime = P::Runtime;
    type Input = P::Input;
    type Artefact = P::Artefact;
    type Finding = P::Finding;
    type Error = P::Error;

    fn repair<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        attempts: Vec<Attempt<Self::Input, Self::Artefact, Self::Finding>>,
    ) -> LocalBoxFuture<'a, Result<Self::Input, Self::Error>> {
        let planner = self.name.clone();
        let inner = &self.inner;
        let span = debug_span!(
            "repair_run",
            component = "repair",
            planner = %planner,
            input_type = %type_name::<P::Input>(),
            artefact_type = %type_name::<P::Artefact>(),
            finding_type = %type_name::<P::Finding>()
        );

        Box::pin(
            async move {
                let attempt_count = attempts.len();
                trace!(action = "input", attempts = ?attempts, attempt_count, "repair attempts");
                debug!(action = "run.start", attempt_count, "repair planner started");

                match inner.repair(runtime, attempts).await {
                    Ok(next_input) => {
                        trace!(action = "output", next_input = ?next_input, "repair planner output");
                        debug!(action = "run.complete", "repair planner completed");
                        Ok(next_input)
                    }
                    Err(error_value) => {
                        trace!(action = "error", error = ?error_value, "repair planner error");
                        error!(action = "run.error", "repair planner failed");
                        Err(error_value)
                    }
                }
            }
            .instrument(span),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fmt::Formatter,
        io::{self, Write},
        sync::{Arc, Mutex},
    };

    use futures::{executor::block_on, future::LocalBoxFuture};
    use serde_json::Value;
    use tracing::{Instrument, Level, subscriber::with_default};

    use super::{CheckExt, MaterialiserExt, RepairPlannerExt, TaskExt};
    use crate::{Attempt, RetryPolicy, Step};

    #[derive(Debug)]
    struct TestRuntime {
        required_revision: usize,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct Input {
        prompt: &'static str,
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

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct TestError;

    impl std::fmt::Display for TestError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            f.write_str("test error")
        }
    }

    impl std::error::Error for TestError {}

    struct Generate;

    impl crate::Task for Generate {
        type Runtime = TestRuntime;
        type Input = Input;
        type Output = Patch;
        type Error = TestError;

        fn run<'a>(
            &'a self,
            _runtime: &'a Self::Runtime,
            input: Self::Input,
        ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
            Box::pin(async move {
                Ok(Patch {
                    revision: input.revision,
                })
            })
        }
    }

    struct ApplyPatch;

    impl crate::Materialiser for ApplyPatch {
        type Runtime = TestRuntime;
        type Input = Patch;
        type Output = Workspace;
        type Error = TestError;

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

    impl crate::Check for CargoTest {
        type Runtime = TestRuntime;
        type Subject = Workspace;
        type Finding = Finding;
        type Error = TestError;

        fn check<'a>(
            &'a self,
            runtime: &'a Self::Runtime,
            subject: Self::Subject,
        ) -> LocalBoxFuture<'a, Result<Vec<Self::Finding>, Self::Error>> {
            Box::pin(async move {
                if subject.revision >= runtime.required_revision {
                    Ok(Vec::new())
                } else {
                    Ok(vec![Finding::TestsFailed])
                }
            })
        }
    }

    struct Repair;

    impl crate::RepairPlanner for Repair {
        type Runtime = TestRuntime;
        type Input = Input;
        type Artefact = Patch;
        type Finding = Finding;
        type Error = TestError;

        fn repair<'a>(
            &'a self,
            _runtime: &'a Self::Runtime,
            attempts: Vec<Attempt<Self::Input, Self::Artefact, Self::Finding>>,
        ) -> LocalBoxFuture<'a, Result<Self::Input, Self::Error>> {
            Box::pin(async move {
                let previous = attempts.last().expect("attempt present");
                Ok(Input {
                    prompt: previous.input.prompt,
                    revision: previous.artefact.revision + 1,
                })
            })
        }
    }

    #[derive(Clone, Default)]
    struct SharedBuffer(Arc<Mutex<Vec<u8>>>);

    impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for SharedBuffer {
        type Writer = SharedWriter;

        fn make_writer(&'a self) -> Self::Writer {
            SharedWriter(self.0.clone())
        }
    }

    struct SharedWriter(Arc<Mutex<Vec<u8>>>);

    impl Write for SharedWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().expect("buffer lock").extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn has_span_field(entry: &Value, field: &str, expected: &str) -> bool {
        entry["span"][field] == expected
            || entry["spans"]
                .as_array()
                .is_some_and(|spans| spans.iter().any(|span| span[field] == expected))
    }

    #[test]
    fn observed_components_emit_structured_json_logs() {
        let buffer = SharedBuffer::default();
        let subscriber = tracing_subscriber::fmt()
            .json()
            .with_ansi(false)
            .with_current_span(true)
            .with_span_list(true)
            .with_max_level(Level::TRACE)
            .with_writer(buffer.clone())
            .finish();

        let step = Step::builder(Generate.observed_as("generate_patch"))
            .materialise(ApplyPatch.observed_as("apply_patch"))
            .validate(CargoTest.observed_as("cargo_test"))
            .repair_with(Repair.observed_as("repair_patch"))
            .retry_policy(RetryPolicy::new(2))
            .build();

        with_default(subscriber, || {
            block_on(async {
                step.run(
                    &TestRuntime {
                        required_revision: 1,
                    },
                    Input {
                        prompt: "fix tests",
                        revision: 0,
                    },
                )
                .instrument(tracing::info_span!("test_run"))
                .await
                .expect("step should succeed");
            });
        });

        let raw = String::from_utf8(buffer.0.lock().expect("buffer lock").clone())
            .expect("utf8 log output");
        let entries: Vec<Value> = raw
            .lines()
            .map(|line| serde_json::from_str(line).expect("valid json log line"))
            .collect();

        assert!(entries.iter().any(|entry| {
            entry["fields"]["action"] == "input"
                && has_span_field(entry, "task", "generate_patch")
                && entry["fields"]["input"].to_string().contains("fix tests")
        }));
        assert!(entries.iter().any(|entry| {
            entry["fields"]["action"] == "output"
                && has_span_field(entry, "materialiser", "apply_patch")
                && entry["fields"]["output"].to_string().contains("revision")
        }));
        assert!(entries.iter().any(|entry| {
            entry["fields"]["action"] == "output"
                && has_span_field(entry, "planner", "repair_patch")
                && entry["fields"]["next_input"]
                    .to_string()
                    .contains("revision")
        }));
    }
}
