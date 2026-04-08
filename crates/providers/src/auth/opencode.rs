//! OpenCode Go authentication configuration.

use super::Auth;

const OPENCODE_BASE_URL: &str = "https://opencode.ai/zen";

/// Authentication configuration for OpenCode Go's API.
pub struct OpenCodeAuth {
    api_key: String,
    base_url: String,
}

impl OpenCodeAuth {
    /// Creates a new OpenCode Go authentication configuration.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: OPENCODE_BASE_URL.to_string(),
        }
    }

    /// Creates a new OpenCode Go authentication configuration with a custom base URL.
    pub fn with_base_url(api_key: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: base_url.into(),
        }
    }

    /// Creates a new OpenCode Go authentication configuration with a custom base URL.
    ///
    /// This is primarily useful for testing with mock servers.
    #[cfg(test)]
    pub fn set_base_url(&mut self, base_url: impl Into<String>) {
        self.base_url = base_url.into();
    }
}

impl Auth for OpenCodeAuth {
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
    fn test_opencode_auth_new() {
        let auth = OpenCodeAuth::new("test-key");
        assert_eq!(auth.base_url(), OPENCODE_BASE_URL);
        assert_eq!(
            auth.auth_header(),
            ("Authorization", "Bearer test-key".to_string())
        );
    }

    #[test]
    fn test_opencode_auth_custom_base_url() {
        let mut auth = OpenCodeAuth::new("test-key");
        auth.set_base_url("https://custom.api.com");
        assert_eq!(auth.base_url(), "https://custom.api.com");
        assert_eq!(
            auth.auth_header(),
            ("Authorization", "Bearer test-key".to_string())
        );
    }
}
