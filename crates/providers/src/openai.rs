//! OpenAI API client.

use std::env;

use naaf_model::{
    GenerationRequest, GenerationResponse, Message, ModelProvider, ProviderCapabilities,
    ProviderError, Result, Usage,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";

pub struct OpenAiConfig {
    pub api_key: String,
    pub base_url: Option<String>,
}

impl OpenAiConfig {
    pub fn from_env() -> Result<Self> {
        let api_key = env::var("OPENAI_API_KEY").map_err(|_| {
            ProviderError::Authentication("OPENAI_API_KEY environment variable not set".into())
        })?;
        Ok(Self {
            api_key,
            base_url: None,
        })
    }
}

#[derive(Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct OpenAiResponse {
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
#[allow(dead_code)]
struct ResponseMessage {
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
#[allow(dead_code)]
struct OpenAiError {
    message: String,
    #[serde(rename = "type")]
    error_type: String,
    code: Option<String>,
}

pub struct OpenAiProvider {
    client: Client,
    config: OpenAiConfig,
}

impl OpenAiProvider {
    pub fn new(config: OpenAiConfig) -> Self {
        let client = Client::new();
        Self { client, config }
    }

    pub fn from_env() -> Result<Self> {
        let config = OpenAiConfig::from_env()?;
        Ok(Self::new(config))
    }

    fn api_url(&self) -> String {
        self.config
            .base_url
            .clone()
            .unwrap_or_else(|| OPENAI_API_URL.to_string())
    }

    fn map_error(&self, status: u16, body: &str) -> ProviderError {
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

impl ModelProvider for OpenAiProvider {
    async fn generate(&self, request: GenerationRequest) -> Result<GenerationResponse> {
        let openai_request = OpenAiRequest {
            model: request.model.clone(),
            messages: request.messages.clone(),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
        };

        let response = self
            .client
            .post(self.api_url())
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&openai_request)
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| ProviderError::ParseError(e.to_string()))?;

        if !status.is_success() {
            return Err(self.map_error(status.as_u16(), &body));
        }

        let openai_response: OpenAiResponse =
            serde_json::from_str(&body).map_err(|e| ProviderError::ParseError(e.to_string()))?;

        let choice = openai_response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| ProviderError::InvalidRequest("No choices in response".into()))?;

        Ok(GenerationResponse {
            content: choice.message.content,
            model: openai_response.model,
            usage: Usage {
                prompt_tokens: openai_response.usage.prompt_tokens,
                completion_tokens: openai_response.usage.completion_tokens,
                total_tokens: openai_response.usage.total_tokens,
            },
            finish_reason: choice.finish_reason,
        })
    }

    async fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::new(false, 128_000).with_models(vec![
            "gpt-4".to_string(),
            "gpt-4-turbo".to_string(),
            "gpt-3.5-turbo".to_string(),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_provider_without_api_key() {
        let result = OpenAiProvider::from_env();
        assert!(result.is_err());
    }

    #[test]
    fn test_openai_config_missing_api_key() {
        let config = OpenAiConfig {
            api_key: "test".to_string(),
            base_url: None,
        };
        let provider = OpenAiProvider::new(config);
        assert_eq!(provider.api_url(), OPENAI_API_URL);
    }
}

#[cfg(test)]
mod integration {
    use super::*;
    use httpmock::MockServer;

    #[tokio::test]
    async fn test_successful_generation_call() {
        let server = MockServer::start();

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

        let config = OpenAiConfig {
            api_key: "test-key".to_string(),
            base_url: Some(server.url("/v1/chat/completions").to_string()),
        };
        let provider = OpenAiProvider::new(config);

        let request = GenerationRequest::new("gpt-4".to_string(), vec![Message::user("Hello")]);

        let result = provider.generate(request).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.content, "Hello! How can I help you today?");
        assert_eq!(response.model, "gpt-4");
        assert_eq!(response.usage.total_tokens, 30);
        assert_eq!(response.finish_reason, "stop");

        _mock.assert();
    }

    #[tokio::test]
    async fn test_api_error_mapping() {
        let server = MockServer::start();

        let error_response = serde_json::json!({
            "error": {
                "message": "Invalid API key",
                "type": "invalid_request_error",
                "code": "invalid_api_key"
            }
        });

        let _mock = server.mock(|when, then| {
            when.method(httpmock::Method::POST)
                .path("/v1/chat/completions");
            then.status(401)
                .header("Content-Type", "application/json")
                .json_body(error_response);
        });

        let config = OpenAiConfig {
            api_key: "invalid-key".to_string(),
            base_url: Some(server.url("/v1/chat/completions").to_string()),
        };
        let provider = OpenAiProvider::new(config);

        let request = GenerationRequest::new("gpt-4".to_string(), vec![Message::user("Hello")]);

        let result = provider.generate(request).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ProviderError::Authentication(_)));

        _mock.assert();
    }
}
