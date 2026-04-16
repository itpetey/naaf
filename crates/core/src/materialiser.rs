use futures::future::LocalBoxFuture;

/// Turns a logical subject into a materialised subject, typically with side effects.
pub trait Materialiser {
    /// The shared runtime capabilities used by this materialiser.
    type Runtime;
    /// The input subject transformed by this materialiser.
    type Input;
    /// The materialised subject produced by this materialiser.
    type Output;
    /// Errors thrown by the materialiser infrastructure that cannot be recovered.
    type Error;

    /// Materialises the input subject using the shared runtime.
    fn materialise<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        input: Self::Input,
    ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>>;
}
