//! Model provider trait.

use std::error::Error;
use std::fmt;

use crate::types::{GenerationRequest, GenerationResponse, ProviderCapabilities};

#[derive(Debug)]
pub enum ProviderError {
    Authentication(String),
    RateLimited(String),
    ModelNotFound(String),
    InvalidRequest(String),
    NetworkError(String),
    ParseError(String),
}

impl fmt::Display for ProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProviderError::Authentication(msg) => write!(f, "Authentication error: {}", msg),
            ProviderError::RateLimited(msg) => write!(f, "Rate limited: {}", msg),
            ProviderError::ModelNotFound(msg) => write!(f, "Model not found: {}", msg),
            ProviderError::InvalidRequest(msg) => write!(f, "Invalid request: {}", msg),
            ProviderError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            ProviderError::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl Error for ProviderError {}

pub type Result<T> = std::result::Result<T, ProviderError>;

pub trait ModelProvider: Send + Sync {
    fn generate(&self, request: GenerationRequest) -> Result<GenerationResponse>;
    fn capabilities(&self) -> ProviderCapabilities;
}
