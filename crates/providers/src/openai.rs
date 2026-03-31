//! OpenAI provider with model enumeration.
//!
//! Provides factory methods for creating OpenAI providers with pre-configured models.

use crate::Provider;
use crate::api::OpenAiChatCompletions;
use crate::auth::OpenAiAuth;

/// OpenAI model identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenAiModel {
    Gpt5,
    Gpt54,
}

impl OpenAiModel {
    /// Creates a provider for GPT 5
    pub fn gpt5(api_key: impl Into<String>) -> Provider<OpenAiAuth, OpenAiChatCompletions> {
        Provider::new(
            OpenAiAuth::new(api_key),
            OpenAiChatCompletions::new(Self::Gpt5),
        )
    }

    /// Creates a provider for GPT 5.4
    pub fn gpt54(api_key: impl Into<String>) -> Provider<OpenAiAuth, OpenAiChatCompletions> {
        Provider::new(
            OpenAiAuth::new(api_key),
            OpenAiChatCompletions::new(Self::Gpt54),
        )
    }
}

impl std::fmt::Display for OpenAiModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            OpenAiModel::Gpt5 => "gpt-5",
            OpenAiModel::Gpt54 => "gpt-54",
        };
        write!(f, "{}", s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::ApiSpec;
    use crate::auth::Auth;

    #[test]
    fn test_provider_method_gpt5() {
        let provider = OpenAiModel::gpt5("test-key");
        assert_eq!(provider.auth().base_url(), "https://api.openai.com");
        assert_eq!(provider.api().model(), "gpt-5");
    }

    #[test]
    fn test_provider_method_gpt54() {
        let provider = OpenAiModel::gpt54("test-key");
        assert_eq!(provider.auth().base_url(), "https://api.openai.com");
        assert_eq!(provider.api().model(), "gpt-54");
    }
}
