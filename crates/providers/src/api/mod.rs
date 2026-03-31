//! API wire format abstractions.

mod anthropic;
mod openai_chat;

use naaf_model::{GenerationResponse, ProviderError};

pub use anthropic::AnthropicMessages;
pub use openai_chat::OpenAiChatCompletions;

/// Trait for LLM API wire formats.
///
/// Defines how to build requests and parse responses for a specific API.
/// Implementations handle the translation between our generic types and
/// API-specific formats.
pub trait ApiSpec: Send + Sync {
    /// Returns the endpoint path for this API (e.g., "/v1/chat/completions").
    fn endpoint(&self) -> &'static str;

    /// Returns the model identifier for this API spec.
    fn model(&self) -> &str;

    /// Returns the capabilities for this provider configuration.
    fn capabilities() -> naaf_model::ProviderCapabilities;

    /// Parses a successful response body into a GenerationResponse.
    fn parse_response(&self, body: &str) -> Result<GenerationResponse, ProviderError>;

    /// Parses an error response into a ProviderError.
    fn parse_error(&self, status: u16, body: &str) -> ProviderError;
}
