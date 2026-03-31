//! Authentication abstractions for LLM providers.

mod openai;
mod opencode;

pub use openai::OpenAiAuth;
pub use opencode::OpenCodeAuth;

/// Trait for provider authentication configuration.
///
/// Provides base URL and authentication headers for API requests.
/// Implementations define provider-specific authentication schemes.
pub trait Auth: Send + Sync {
    /// Returns the base URL for API requests.
    fn base_url(&self) -> &str;

    /// Returns the authentication header name and value.
    ///
    /// Returns a tuple of (header_name, header_value), e.g., ("Authorization", "Bearer abc123").
    fn auth_header(&self) -> (&'static str, String);
}
