use futures::future::LocalBoxFuture;

/// Observes a subject and returns zero or more findings.
pub trait Check {
    /// The shared runtime capabilities used by this check.
    type Runtime;
    /// The value observed by this check.
    type Subject;
    /// Findings reported when the check does not pass cleanly.
    type Finding;
    /// Errors thrown by the check infrastructure that cannot be recovered.
    type Error;

    /// Runs the check against the given subject.
    fn check<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        subject: Self::Subject,
    ) -> LocalBoxFuture<'a, Result<Vec<Self::Finding>, Self::Error>>;
}
