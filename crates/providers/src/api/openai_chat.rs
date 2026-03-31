//! OpenAI Chat Completions API specification.

use std::fmt::Display;

use naaf_model::{GenerationResponse, ProviderCapabilities, ProviderError, Usage};
use serde::{Deserialize, Serialize};

use super::ApiSpec;

/// OpenAI Chat Completions API specification.
///
/// Implements the `/v1/chat/completions` endpoint used by OpenAI,
/// OpenCode Go, and other OpenAI-compatible providers.
pub struct OpenAiChatCompletions {
    model: String,
}

impl OpenAiChatCompletions {
    /// Creates a new OpenAI Chat Completions specification for the given model.
    pub fn new(model: impl Display) -> Self {
        Self {
            model: model.to_string(),
        }
    }
}

impl ApiSpec for OpenAiChatCompletions {
    fn endpoint(&self) -> &'static str {
        "/v1/chat/completions"
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn capabilities() -> ProviderCapabilities {
        ProviderCapabilities::new(false, 128_000).with_models(vec![
            "gpt-4".to_string(),
            "gpt-4-turbo".to_string(),
            "gpt-3.5-turbo".to_string(),
        ])
    }

    fn parse_response(&self, body: &str) -> Result<GenerationResponse, ProviderError> {
        let response: OpenAiResponse =
            serde_json::from_str(body).map_err(|e| ProviderError::ParseError(e.to_string()))?;

        let choice = response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| ProviderError::InvalidRequest("No choices in response".into()))?;

        Ok(GenerationResponse {
            content: choice.message.content,
            model: response.model,
            usage: Usage {
                prompt_tokens: response.usage.prompt_tokens,
                completion_tokens: response.usage.completion_tokens,
                total_tokens: response.usage.total_tokens,
            },
            finish_reason: choice.finish_reason,
        })
    }

    fn parse_error(&self, status: u16, body: &str) -> ProviderError {
        if let Ok(error_resp) = serde_json::from_str::<OpenAiErrorResponse>(body) {
            let msg = error_resp.error.message;
            match status {
                401 => ProviderError::Authentication(msg),
                404 => ProviderError::ModelNotFound(msg),
                429 => ProviderError::RateLimited(msg),
                400..=499 => ProviderError::InvalidRequest(msg),
                500..=599 => ProviderError::NetworkError(msg),
                _ => ProviderError::NetworkError(msg),
            }
        } else {
            ProviderError::ParseError(format!("Failed to parse error response: {}", body))
        }
    }
}

/// OpenAI Chat Completions request format.
#[allow(dead_code)]
#[derive(Serialize)]
pub struct OpenAiRequest {
    pub model: String,
    pub messages: Vec<naaf_model::Message>,
    pub temperature: f32,
    pub max_tokens: u32,
}

/// OpenAI Chat Completions response format.
#[derive(Deserialize)]
struct OpenAiResponse {
    #[allow(dead_code)]
    id: String,
    model: String,
    choices: Vec<Choice>,
    usage: UsageResponse,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
    finish_reason: String,
}

#[derive(Deserialize)]
struct ResponseMessage {
    #[allow(dead_code)]
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct UsageResponse {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Deserialize)]
struct OpenAiErrorResponse {
    error: OpenAiError,
}

#[derive(Deserialize)]
struct OpenAiError {
    message: String,
    #[allow(dead_code)]
    #[serde(rename = "type")]
    error_type: String,
    #[allow(dead_code)]
    code: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_endpoint() {
        let spec = OpenAiChatCompletions::new("gpt-4");
        assert_eq!(spec.endpoint(), "/v1/chat/completions");
    }

    #[test]
    fn test_model() {
        let spec = OpenAiChatCompletions::new("gpt-4");
        assert_eq!(spec.model(), "gpt-4");
    }

    #[test]
    fn test_parse_response() {
        let spec = OpenAiChatCompletions::new("gpt-4");
        let body = r#"{"id":"chatcmpl-123","model":"gpt-4","choices":[{"message":{"role":"assistant","content":"Hello!"},"finish_reason":"stop"}],"usage":{"prompt_tokens":10,"completion_tokens":5,"total_tokens":15}}"#;
        let response = spec.parse_response(body).unwrap();
        assert_eq!(response.content, "Hello!");
        assert_eq!(response.model, "gpt-4");
        assert_eq!(response.finish_reason, "stop");
        assert_eq!(response.usage.total_tokens, 15);
    }

    #[test]
    fn test_parse_error() {
        let spec = OpenAiChatCompletions::new("gpt-4");
        let body = r#"{"error":{"message":"Invalid API key","type":"invalid_request_error"}}"#;
        let error = spec.parse_error(401, body);
        assert!(matches!(error, ProviderError::Authentication(_)));
    }
}
