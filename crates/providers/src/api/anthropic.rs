//! Anthropic Messages API specification.

use std::fmt::Display;

use serde::{Deserialize, Serialize};

use super::ApiSpec;
use crate::{GenerationResponse, Message, ProviderCapabilities, ProviderError, Usage};

/// Anthropic Messages API specification.
///
/// Implements the `/v1/messages` endpoint used by Anthropic and Anthropic-compatible providers.
pub struct AnthropicMessages {
    model: String,
}

impl AnthropicMessages {
    /// Creates a new Anthropic Messages specification for the given model.
    pub fn new(model: impl Display) -> Self {
        Self {
            model: model.to_string(),
        }
    }
}

impl ApiSpec for AnthropicMessages {
    fn endpoint(&self) -> &'static str {
        "/v1/messages"
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn capabilities() -> ProviderCapabilities {
        ProviderCapabilities::new(false, 200_000).with_models(vec![
            "minimax-m2.5".to_string(),
            "minimax-m2.7".to_string(),
            "claude-3-opus".to_string(),
            "claude-3-sonnet".to_string(),
            "claude-3-haiku".to_string(),
        ])
    }

    fn parse_response(&self, body: &str) -> Result<GenerationResponse, ProviderError> {
        let response: AnthropicResponse =
            serde_json::from_str(body).map_err(|e| ProviderError::ParseError(e.to_string()))?;

        let content_block = response
            .content
            .into_iter()
            .find(|c| c.type_field == "text")
            .ok_or_else(|| ProviderError::InvalidRequest("No text content in response".into()))?;

        Ok(GenerationResponse {
            content: content_block.text,
            model: response.model,
            usage: Usage {
                prompt_tokens: response.usage.input_tokens,
                completion_tokens: response.usage.output_tokens,
                total_tokens: response.usage.input_tokens + response.usage.output_tokens,
            },
            finish_reason: response.stop_reason,
        })
    }

    fn parse_error(&self, status: u16, body: &str) -> ProviderError {
        if let Ok(error_resp) = serde_json::from_str::<AnthropicErrorResponse>(body) {
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

/// Anthropic Messages request format.
#[allow(dead_code)]
#[derive(Serialize)]
pub struct AnthropicRequest {
    pub model: String,
    pub messages: Vec<AnthropicMessage>,
    #[serde(rename = "max_tokens")]
    pub max_tokens: u32,
}

/// Anthropic message format.
#[allow(dead_code)]
#[derive(Serialize)]
pub struct AnthropicMessage {
    pub role: String,
    pub content: String,
}

impl From<Message> for AnthropicMessage {
    fn from(msg: Message) -> Self {
        Self {
            role: msg.role,
            content: msg.content,
        }
    }
}

/// Anthropic Messages response format.
#[derive(Deserialize)]
struct AnthropicResponse {
    #[allow(dead_code)]
    id: String,
    model: String,
    content: Vec<ContentBlock>,
    usage: AnthropicUsage,
    stop_reason: String,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    type_field: String,
    text: String,
}

#[derive(Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Deserialize)]
struct AnthropicErrorResponse {
    error: AnthropicError,
}

#[derive(Deserialize)]
struct AnthropicError {
    message: String,
    #[allow(dead_code)]
    #[serde(rename = "type")]
    error_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_endpoint() {
        let spec = AnthropicMessages::new("minimax-m2.5");
        assert_eq!(spec.endpoint(), "/v1/messages");
    }

    #[test]
    fn test_model() {
        let spec = AnthropicMessages::new("minimax-m2.7");
        assert_eq!(spec.model(), "minimax-m2.7");
    }

    #[test]
    fn test_parse_response() {
        let spec = AnthropicMessages::new("minimax-m2.5");
        let body = r#"{"id":"msg-123","model":"minimax-m2.5","content":[{"type":"text","text":"Hello!"}],"usage":{"input_tokens":10,"output_tokens":5},"stop_reason":"end_turn"}"#;
        let response = spec.parse_response(body).unwrap();
        assert_eq!(response.content, "Hello!");
        assert_eq!(response.model, "minimax-m2.5");
        assert_eq!(response.finish_reason, "end_turn");
        assert_eq!(response.usage.total_tokens, 15);
    }

    #[test]
    fn test_parse_error() {
        let spec = AnthropicMessages::new("minimax-m2.5");
        let body = r#"{"error":{"message":"Invalid API key","type":"authentication_error"}}"#;
        let error = spec.parse_error(401, body);
        assert!(matches!(error, ProviderError::Authentication(_)));
    }
}
