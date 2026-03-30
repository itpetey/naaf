//! Model: provider trait and shared request/response types.

pub mod provider;
pub mod types;

pub use provider::{ModelProvider, ProviderError, Result};
pub use types::{GenerationRequest, GenerationResponse, Message, ProviderCapabilities, Usage};
