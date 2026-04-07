//! Common request/response types.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub temperature: f32,
    pub max_tokens: u32,
}

impl GenerationRequest {
    pub fn new(model: String, messages: Vec<Message>) -> Self {
        Self {
            model,
            messages,
            temperature: 0.7,
            max_tokens: 1024,
        }
    }

    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl Message {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationResponse {
    pub content: String,
    pub model: String,
    pub usage: Usage,
    pub finish_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    pub supports_streaming: bool,
    pub max_context_tokens: u32,
    pub supported_models: Vec<String>,
}

impl ProviderCapabilities {
    pub fn new(supports_streaming: bool, max_context_tokens: u32) -> Self {
        Self {
            supports_streaming,
            max_context_tokens,
            supported_models: Vec::new(),
        }
    }

    pub fn with_models(mut self, models: Vec<String>) -> Self {
        self.supported_models = models;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generation_request_serialization() {
        let request = GenerationRequest::new("gpt-4".to_string(), vec![Message::user("Hello")]);
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("gpt-4"));
        assert!(json.contains("user"));
        assert!(json.contains("Hello"));
    }

    #[test]
    fn test_generation_request_with_custom_params() {
        let request = GenerationRequest::new(
            "gpt-4".to_string(),
            vec![Message::system("You are helpful.")],
        )
        .with_temperature(0.9)
        .with_max_tokens(500);

        assert_eq!(request.temperature, 0.9);
        assert_eq!(request.max_tokens, 500);
    }

    #[test]
    fn test_message_user() {
        let msg = Message::user("hello");
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "hello");
    }

    #[test]
    fn test_message_system() {
        let msg = Message::system("You are helpful.");
        assert_eq!(msg.role, "system");
        assert_eq!(msg.content, "You are helpful.");
    }

    #[test]
    fn test_generation_response_serialization() {
        let response = GenerationResponse {
            content: "Test response".to_string(),
            model: "gpt-4".to_string(),
            usage: Usage {
                prompt_tokens: 10,
                completion_tokens: 20,
                total_tokens: 30,
            },
            finish_reason: "stop".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("Test response"));
        assert!(json.contains("gpt-4"));
    }

    #[test]
    fn test_provider_capabilities() {
        let caps = ProviderCapabilities::new(true, 128_000)
            .with_models(vec!["gpt-4".to_string(), "gpt-3.5-turbo".to_string()]);

        assert!(caps.supports_streaming);
        assert_eq!(caps.max_context_tokens, 128_000);
        assert_eq!(caps.supported_models.len(), 2);
    }
}
