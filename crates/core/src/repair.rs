use std::fmt::{Debug, Formatter};

use futures::future::LocalBoxFuture;
use serde::{Deserialize, Serialize};

/// Produces the next task input from earlier failed attempts.
pub trait RepairPlanner {
    /// The shared runtime capabilities used by this planner.
    type Runtime;
    /// The next task input to produce.
    type Input;
    /// The step output produced by the task.
    type Output;
    /// Findings gathered from checks.
    type Finding;
    /// Errors thrown by the planner infrastructure that cannot be recovered.
    type Error;

    /// Plans the next input after one or more failed attempts.
    fn repair<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        attempts: Vec<Attempt<Self::Input, Self::Output, Self::Finding>>,
    ) -> LocalBoxFuture<'a, Result<Self::Input, Self::Error>>;
}

/// One failed step attempt captured for repair planning.
#[derive(Clone, PartialEq, Eq)]
pub struct Attempt<I, O, F> {
    /// The input that produced this attempt.
    pub input: I,
    /// The output produced by the task before repair.
    pub output: O,
    /// Findings gathered from checks for this attempt.
    pub findings: Vec<F>,
}

/// Configures how many attempts a step may perform before it is rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryPolicy {
    max_attempts: Option<usize>,
}

/// A lightweight view of one step attempt recorded in a report.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttemptReport<F> {
    /// Findings produced by checks for this attempt.
    pub findings: Vec<F>,
    /// Whether this attempt was accepted and ended the step successfully.
    pub accepted: bool,
}

/// A summary of all attempts performed by a step.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepReport<F> {
    attempts: Vec<AttemptReport<F>>,
}

/// Successful step output paired with attempt metadata.
#[derive(Clone, PartialEq, Eq)]
pub struct Traced<T, F> {
    output: T,
    report: StepReport<F>,
}

/// Placeholder finding type for steps that do not yet bind a finding type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NeverFinding {}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::new(1)
    }
}

impl<I, O, F> Debug for Attempt<I, O, F>
where
    I: Debug,
    O: Debug,
    F: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Attempt")
            .field("input", &self.input)
            .field("output", &self.output)
            .field("findings", &self.findings)
            .finish()
    }
}

impl RetryPolicy {
    /// Creates a retry policy with the given maximum number of attempts.
    pub fn new(max_attempts: usize) -> Self {
        assert!(
            max_attempts > 0,
            "retry policy must allow at least one attempt"
        );
        Self {
            max_attempts: Some(max_attempts),
        }
    }

    /// Creates a retry policy with no attempt limit.
    pub fn unlimited() -> Self {
        Self { max_attempts: None }
    }

    /// Returns the maximum number of attempts permitted for a step, if finite.
    pub fn max_attempts(self) -> Option<usize> {
        self.max_attempts
    }

    /// Returns whether the policy has no attempt limit.
    pub fn is_unlimited(self) -> bool {
        self.max_attempts.is_none()
    }

    /// Returns whether the given attempt count has exhausted this policy.
    pub fn is_exhausted(self, attempt_count: usize) -> bool {
        self.max_attempts
            .is_some_and(|max_attempts| attempt_count >= max_attempts)
    }
}

#[cfg(test)]
mod tests {
    use super::RetryPolicy;

    #[test]
    fn finite_retry_policy_exhausts_at_max_attempts() {
        let policy = RetryPolicy::new(3);

        assert_eq!(policy.max_attempts(), Some(3));
        assert!(!policy.is_exhausted(2));
        assert!(policy.is_exhausted(3));
    }

    #[test]
    fn unlimited_retry_policy_never_exhausts() {
        let policy = RetryPolicy::unlimited();

        assert_eq!(policy.max_attempts(), None);
        assert!(policy.is_unlimited());
        assert!(!policy.is_exhausted(usize::MAX));
    }
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

    /// Maps every recorded finding while preserving attempt acceptance metadata.
    pub fn map_findings<NextFinding>(
        self,
        map: impl Fn(F) -> NextFinding,
    ) -> StepReport<NextFinding> {
        StepReport {
            attempts: self
                .attempts
                .into_iter()
                .map(|attempt| AttemptReport {
                    findings: attempt.findings.into_iter().map(&map).collect(),
                    accepted: attempt.accepted,
                })
                .collect(),
        }
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
