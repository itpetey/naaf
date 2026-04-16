use std::fmt::{Debug, Formatter};

use futures::future::LocalBoxFuture;

/// One failed step attempt captured for repair planning.
#[derive(Clone, PartialEq, Eq)]
pub struct Attempt<I, A, F> {
    /// The input that produced this attempt.
    pub input: I,
    /// The artefact produced by the task before repair.
    pub artefact: A,
    /// Findings gathered from checks for this attempt.
    pub findings: Vec<F>,
}

impl<I, A, F> Debug for Attempt<I, A, F>
where
    I: Debug,
    A: Debug,
    F: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Attempt")
            .field("input", &self.input)
            .field("artefact", &self.artefact)
            .field("findings", &self.findings)
            .finish()
    }
}

/// Produces the next task input from earlier failed attempts.
pub trait RepairPlanner {
    /// The shared runtime capabilities used by this planner.
    type Runtime;
    /// The next task input to produce.
    type Input;
    /// The step artefact produced by the task.
    type Artefact;
    /// Findings gathered from checks.
    type Finding;
    /// Errors thrown by the planner infrastructure that cannot be recovered.
    type Error;

    /// Plans the next input after one or more failed attempts.
    fn repair<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        attempts: Vec<Attempt<Self::Input, Self::Artefact, Self::Finding>>,
    ) -> LocalBoxFuture<'a, Result<Self::Input, Self::Error>>;
}

/// Configures how many attempts a step may perform before it is rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetryPolicy {
    max_attempts: usize,
}

impl RetryPolicy {
    /// Creates a retry policy with the given maximum number of attempts.
    pub fn new(max_attempts: usize) -> Self {
        assert!(
            max_attempts > 0,
            "retry policy must allow at least one attempt"
        );
        Self { max_attempts }
    }

    /// Returns the maximum number of attempts permitted for a step.
    pub fn max_attempts(self) -> usize {
        self.max_attempts
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::new(1)
    }
}

/// A lightweight view of one step attempt recorded in a report.
#[derive(Clone, PartialEq, Eq)]
pub struct AttemptReport<F> {
    /// Findings produced by checks for this attempt.
    pub findings: Vec<F>,
    /// Whether this attempt was accepted and ended the step successfully.
    pub accepted: bool,
}

impl<F> AttemptReport<F> {
    /// Returns whether this attempt was accepted.
    pub fn accepted(&self) -> bool {
        self.accepted
    }
}

impl<F> Debug for AttemptReport<F>
where
    F: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AttemptReport")
            .field("findings", &self.findings)
            .field("accepted", &self.accepted)
            .finish()
    }
}

/// A summary of all attempts performed by a step.
#[derive(Clone, PartialEq, Eq)]
pub struct StepReport<F> {
    attempts: Vec<AttemptReport<F>>,
}

impl<F> StepReport<F> {
    /// Creates a report from prebuilt attempt summaries.
    pub fn new(attempts: Vec<AttemptReport<F>>) -> Self {
        Self { attempts }
    }

    /// Returns the attempts recorded in this report.
    pub fn attempts(&self) -> &[AttemptReport<F>] {
        &self.attempts
    }

    /// Returns the number of attempts recorded in this report.
    pub fn attempt_count(&self) -> usize {
        self.attempts.len()
    }

    /// Appends another report to this one.
    pub fn extend(mut self, mut other: Self) -> Self {
        self.attempts.append(&mut other.attempts);
        self
    }
}

impl<F> Debug for StepReport<F>
where
    F: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StepReport")
            .field("attempts", &self.attempts)
            .finish()
    }
}

/// Successful step output paired with attempt metadata.
#[derive(Clone, PartialEq, Eq)]
pub struct Traced<T, F> {
    output: T,
    report: StepReport<F>,
}

impl<T, F> Traced<T, F> {
    /// Creates a traced value from an output and step report.
    pub fn new(output: T, report: StepReport<F>) -> Self {
        Self { output, report }
    }

    /// Returns the successful output.
    pub fn output(&self) -> &T {
        &self.output
    }

    /// Returns the report collected while producing the output.
    pub fn report(&self) -> &StepReport<F> {
        &self.report
    }

    /// Discards the report and returns only the output.
    pub fn into_output(self) -> T {
        self.output
    }

    /// Splits the traced value into its output and report.
    pub fn into_parts(self) -> (T, StepReport<F>) {
        (self.output, self.report)
    }
}

impl<T, F> Debug for Traced<T, F>
where
    T: Debug,
    F: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Traced")
            .field("output", &self.output)
            .field("report", &self.report)
            .finish()
    }
}

/// Placeholder finding type for steps that do not yet bind a finding type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NeverFinding {}
