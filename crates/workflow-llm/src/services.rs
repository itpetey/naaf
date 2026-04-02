//! LLM service implementation for workflow runtime.
//!
//! Bridges the ModelProvider trait to the Services trait.

use std::sync::Arc;

use workflow_core::budget::Services;

use naaf_model::{GenerationRequest, Message, ModelProvider};

pub struct LlmServices<P: ModelProvider> {
    provider: Arc<P>,
    model: String,
}

impl<P: ModelProvider> LlmServices<P> {
    pub fn new(provider: Arc<P>, model: String) -> Self {
        Self { provider, model }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LlmServiceError {
    #[error("LLM service error: {0}")]
    Service(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Provider error: {0}")]
    Provider(String),
}

impl<P: ModelProvider + Send + Sync> Services for LlmServices<P> {
    type Error = LlmServiceError;

    async fn call(&self, service: &str, request: &[u8]) -> Result<Vec<u8>, Self::Error> {
        match service {
            "llm" | "model" | "provider" => {
                let prompt = String::from_utf8_lossy(request);
                let generation_request = GenerationRequest::new(
                    self.model.clone(),
                    vec![Message::user(prompt.to_string())],
                );

                let response = self
                    .provider
                    .generate(generation_request)
                    .await
                    .map_err(|e| LlmServiceError::Provider(e.to_string()))?;

                Ok(response.content.into_bytes())
            }
            _ => Err(LlmServiceError::Service(format!(
                "Unknown service: {}",
                service
            ))),
        }
    }
}
