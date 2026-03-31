//! OpenCode Go provider with model enumeration.
//!
//! Provides factory methods for creating OpenCode Go providers with pre-configured models.
//! OpenCode Go supports both OpenAI-compatible and Anthropic-compatible APIs.

use crate::{
    Provider,
    api::{AnthropicMessages, OpenAiChatCompletions},
    auth::OpenCodeAuth,
};

/// OpenCode Go model identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenCodeGoModel {
    Glm5,
    KimiK25,
    MiniMaxM25,
    MiniMaxM27,
}

impl std::fmt::Display for OpenCodeGoModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            OpenCodeGoModel::Glm5 => "glm-5",
            OpenCodeGoModel::KimiK25 => "kimi-k2.5",
            OpenCodeGoModel::MiniMaxM25 => "minimax-m2.5",
            OpenCodeGoModel::MiniMaxM27 => "minimax-m2.7",
        };
        write!(f, "{}", s)
    }
}

impl OpenCodeGoModel {
    /// Creates a provider for GLM-5
    pub fn glm5(api_key: impl Into<String>) -> Provider<OpenCodeAuth, OpenAiChatCompletions> {
        Provider::new(
            OpenCodeAuth::new(api_key),
            OpenAiChatCompletions::new(Self::Glm5),
        )
    }

    /// Creates a provider for Kimi-K2.5
    pub fn kimik25(api_key: impl Into<String>) -> Provider<OpenCodeAuth, OpenAiChatCompletions> {
        Provider::new(
            OpenCodeAuth::new(api_key),
            OpenAiChatCompletions::new(Self::KimiK25),
        )
    }

    /// Creates a provider for MiniMax M2.5
    pub fn minimaxm25(api_key: impl Into<String>) -> Provider<OpenCodeAuth, AnthropicMessages> {
        Provider::new(
            OpenCodeAuth::new(api_key),
            AnthropicMessages::new(Self::MiniMaxM25),
        )
    }

    /// Creates a provider for MiniMax M2.7
    pub fn minimaxm27(api_key: impl Into<String>) -> Provider<OpenCodeAuth, AnthropicMessages> {
        Provider::new(
            OpenCodeAuth::new(api_key),
            AnthropicMessages::new(Self::MiniMaxM27),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::ApiSpec;
    use crate::auth::Auth;

    #[test]
    fn test_provider_method_glm5() {
        let provider = OpenCodeGoModel::glm5("test-key");
        assert_eq!(provider.auth().base_url(), "https://opencode.ai/zen");
        assert_eq!(provider.api().model(), "glm-5");
    }

    #[test]
    fn test_provider_method_kimi_k25() {
        let provider = OpenCodeGoModel::kimik25("test-key");
        assert_eq!(provider.api().model(), "kimi-k2.5");
    }

    #[test]
    fn test_provider_method_minimax_m25() {
        let provider = OpenCodeGoModel::minimaxm25("test-key");

        assert_eq!(provider.api().model(), "minimax-m2.5");
        assert_eq!(provider.api().endpoint(), "/v1/messages");
    }

    #[test]
    fn test_provider_method_minimax_m27() {
        let provider = OpenCodeGoModel::minimaxm27("test-key");
        assert_eq!(provider.api().model(), "minimax-m2.7");
        assert_eq!(provider.api().endpoint(), "/v1/messages");
    }
}

#[cfg(test)]
mod integration {
    use super::*;
    use naaf_model::{GenerationRequest, Message, ModelProvider};

    #[tokio::test]
    async fn test_glm5_generate_success() {
        let server = httpmock::MockServer::start();

        let mock_response = serde_json::json!({
            "id": "chatcmpl-123",
            "model": "glm-5",
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "Hello from GLM-5!"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 20,
                "total_tokens": 30
            }
        });

        let mock = server.mock(|when, then| {
            when.method(httpmock::Method::POST)
                .path("/v1/chat/completions")
                .header("Authorization", "Bearer test-key");
            then.status(200)
                .header("Content-Type", "application/json")
                .json_body(mock_response);
        });

        let mut auth = OpenCodeAuth::new("test-key");
        auth.set_base_url(server.url(""));
        let api = OpenAiChatCompletions::new(OpenCodeGoModel::Glm5);
        let provider = Provider::new(auth, api);

        let request = GenerationRequest::new("glm-5".to_string(), vec![Message::user("Hello")]);
        let response = provider.generate(request).await.unwrap();

        assert_eq!(response.content, "Hello from GLM-5!");

        mock.assert();
    }
}
