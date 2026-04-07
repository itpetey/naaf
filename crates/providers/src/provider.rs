//! Generic provider composition.

use std::error::Error;
use std::fmt;

use crate::types::{GenerationRequest, GenerationResponse, ProviderCapabilities};

use crate::api::ApiSpec;
use crate::auth::Auth;

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
            Self::Authentication(msg) => write!(f, "Authentication error: {}", msg),
            Self::RateLimited(msg) => write!(f, "Rate limited: {}", msg),
            Self::ModelNotFound(msg) => write!(f, "Model not found: {}", msg),
            Self::InvalidRequest(msg) => write!(f, "Invalid request: {}", msg),
            Self::NetworkError(msg) => write!(f, "Network error: {}", msg),
            Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl Error for ProviderError {}

pub type Result<T> = std::result::Result<T, ProviderError>;

pub trait ModelProvider: Send + Sync {
    fn generate(
        &self,
        request: GenerationRequest,
    ) -> impl Future<Output = Result<GenerationResponse>> + Send;

    fn capabilities(&self) -> impl Future<Output = ProviderCapabilities> + Send;
}

/// A generic LLM provider that composes authentication and API specification.
///
/// This struct combines an `Auth` implementation (providing base URL and
/// authentication headers) with an `ApiSpec` implementation (providing
/// request/response formats and endpoint paths).
///
/// # Type Parameters
///
/// * `A` - Authentication type implementing [`Auth`]
/// * `S` - API specification type implementing [`ApiSpec`]
pub struct Provider<A: Auth, S: ApiSpec> {
    auth: A,
    api: S,
}

impl<A: Auth, S: ApiSpec> Provider<A, S> {
    /// Creates a new provider with the given authentication and API specification.
    pub fn new(auth: A, api: S) -> Self {
        Self { auth, api }
    }

    /// Returns a reference to the authentication configuration.
    pub fn auth(&self) -> &A {
        &self.auth
    }

    /// Returns a mutable reference to the authentication configuration.
    #[cfg(test)]
    pub fn auth_mut(&mut self) -> &mut A {
        &mut self.auth
    }

    /// Returns a reference to the API specification.
    pub fn api(&self) -> &S {
        &self.api
    }
}

impl<A: Auth + Send + Sync, S: ApiSpec + Send + Sync> ModelProvider for Provider<A, S> {
    async fn generate(&self, request: GenerationRequest) -> Result<GenerationResponse> {
        let url = format!("{}{}", self.auth.base_url(), self.api.endpoint());

        let response = reqwest::Client::new()
            .post(&url)
            .header(self.auth.auth_header().0, self.auth.auth_header().1)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| ProviderError::ParseError(e.to_string()))?;

        if !status.is_success() {
            return Err(self.api.parse_error(status.as_u16(), &body));
        }

        self.api.parse_response(&body)
    }

    async fn capabilities(&self) -> ProviderCapabilities {
        S::capabilities()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::OpenAiChatCompletions;
    use crate::auth::OpenAiAuth;

    #[test]
    fn test_provider_new() {
        let auth = OpenAiAuth::new("test-key");
        let api = OpenAiChatCompletions::new("gpt-4");
        let _provider = Provider::new(auth, api);
    }

    #[test]
    fn test_provider_composition_types() {
        let auth = OpenAiAuth::new("test-key");
        let api = OpenAiChatCompletions::new("gpt-4");
        let provider = Provider::new(auth, api);

        fn assert_model_provider<P: ModelProvider>(_: &P) {}
        assert_model_provider(&provider);
    }
}

#[cfg(test)]
mod integration {
    use super::*;
    use crate::Message;
    use crate::api::OpenAiChatCompletions;
    use crate::auth::OpenAiAuth;

    #[tokio::test]
    async fn test_provider_generate_success() {
        let server = httpmock::MockServer::start();

        let mock_response = serde_json::json!({
            "id": "chatcmpl-123",
            "model": "gpt-4",
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "Hello! How can I help you today?"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 20,
                "total_tokens": 30
            }
        });

        let _mock = server.mock(|when, then| {
            when.method(httpmock::Method::POST)
                .path("/v1/chat/completions")
                .header("Authorization", "Bearer test-key");
            then.status(200)
                .header("Content-Type", "application/json")
                .json_body(mock_response);
        });

        let auth = OpenAiAuth::with_base_url("test-key", server.url(""));
        let api = OpenAiChatCompletions::new("gpt-4");
        let provider = Provider::new(auth, api);

        let request = GenerationRequest::new("gpt-4".to_string(), vec![Message::user("Hello")]);

        let result = provider.generate(request).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.content, "Hello! How can I help you today?");
        assert_eq!(response.model, "gpt-4");
        assert_eq!(response.usage.total_tokens, 30);

        _mock.assert();
    }
}
