use futures::future::LocalBoxFuture;

/// Observes a task input/output pair and returns zero or more findings.
pub trait Check {
    /// The shared runtime capabilities used by this check.
    type Runtime;
    /// The task input that produced the observed output.
    type Input;
    /// The task output observed by this check.
    type Output;
    /// Findings reported when the check does not pass cleanly.
    type Finding;
    /// Errors thrown by the check infrastructure that cannot be recovered.
    type Error;

    /// Runs the check against the given input and output.
    fn check<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        input: Self::Input,
        output: Self::Output,
    ) -> LocalBoxFuture<'a, Result<Vec<Self::Finding>, Self::Error>>;
}
