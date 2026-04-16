use futures::future::LocalBoxFuture;

use crate::message::{CompletionRequest, CompletionResponse};

/// Provider-neutral client interface for one model turn.
pub trait LlmClient {
    /// Shared runtime capabilities used by the client.
    type Runtime;
    /// Errors thrown by the provider integration.
    type Error;

    /// Executes one completion request and returns one assistant response.
    fn complete<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        request: CompletionRequest,
    ) -> LocalBoxFuture<'a, Result<CompletionResponse, Self::Error>>;
}
