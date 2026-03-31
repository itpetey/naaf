//! OpenAI authentication configuration.

use super::Auth;

const OPENAI_BASE_URL: &str = "https://api.openai.com";

/// Authentication configuration for OpenAI's API.
pub struct OpenAiAuth {
    api_key: String,
    base_url: String,
}

impl OpenAiAuth {
    /// Creates a new OpenAI authentication configuration with the default base URL.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: OPENAI_BASE_URL.to_string(),
        }
    }

    /// Creates a new OpenAI authentication configuration with a custom base URL.
    pub fn with_base_url(api_key: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: base_url.into(),
        }
    }
}

impl Auth for OpenAiAuth {
    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn auth_header(&self) -> (&'static str, String) {
        ("Authorization", format!("Bearer {}", self.api_key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_auth_new() {
        let auth = OpenAiAuth::new("test-key");
        assert_eq!(auth.base_url(), OPENAI_BASE_URL);
        assert_eq!(
            auth.auth_header(),
            ("Authorization", "Bearer test-key".to_string())
        );
    }

    #[test]
    fn test_openai_auth_custom_base_url() {
        let auth = OpenAiAuth::with_base_url("test-key", "https://custom.api.com");
        assert_eq!(auth.base_url(), "https://custom.api.com");
        assert_eq!(
            auth.auth_header(),
            ("Authorization", "Bearer test-key".to_string())
        );
    }
}
