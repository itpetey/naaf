use std::borrow::Cow;

use futures::future::LocalBoxFuture;

/// Produces a typed output from a typed input using the shared runtime.
pub trait Task {
    /// The shared runtime capabilities used by this task.
    type Runtime;
    /// The data this task operates on.
    type Input;
    /// The successful task output.
    type Output;
    /// Errors thrown by the task infrastructure that cannot be recovered.
    type Error;

    /// Executes the task for one attempt.
    fn run<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        input: Self::Input,
    ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>>;

    /// Returns a human-facing label for UI surfaces when one is available.
    fn label(&self) -> Option<Cow<'static, str>> {
        None
    }
}
