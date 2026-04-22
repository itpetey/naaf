use std::marker::PhantomData;

use futures::future::LocalBoxFuture;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::message::ToolSpec;
use crate::tool::Tool;

const DEFAULT_MAX_RESULTS: usize = 5;

/// Parameters accepted by [`WebSearchTool`].
#[derive(Clone, Debug, Deserialize)]
pub struct WebSearchParams {
    /// Free-form search query text.
    pub query: String,
    #[serde(default = "default_max_results")]
    /// Maximum number of results requested from the backing service.
    pub max_results: usize,
}

#[derive(Serialize)]
struct SearchQuery<'a> {
    q: &'a str,
    count: usize,
}

/// Errors returned by [`WebSearchTool`].
#[derive(Debug, Error)]
pub enum WebSearchError {
    /// The HTTP request to the search service failed.
    #[error("web search request failed: {0}")]
    Http(#[from] reqwest::Error),
    /// Tool arguments could not be deserialised.
    #[error("invalid arguments: {0}")]
    InvalidArguments(String),
}

/// Tool that proxies a simple web search HTTP endpoint.
pub struct WebSearchTool<R> {
    client: Client,
    base_url: String,
    api_key: Option<String>,
    _marker: PhantomData<R>,
}

impl<R> WebSearchTool<R> {
    /// Creates a search tool targeting the given endpoint.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.into(),
            api_key: None,
            _marker: PhantomData,
        }
    }

    /// Configures a bearer token used for outbound requests.
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }
}

impl<R> Tool for WebSearchTool<R> {
    type Runtime = R;
    type Error = WebSearchError;

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "web_search".to_string(),
            description:
                "Search the web for information. Returns search results relevant to the query."
                    .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query",
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum number of results to return",
                    },
                },
                "required": ["query"],
            }),
        }
    }

    fn call<'a>(
        &'a self,
        _runtime: &'a Self::Runtime,
        arguments: Value,
    ) -> LocalBoxFuture<'a, Result<Value, Self::Error>> {
        Box::pin(async move {
            let params: WebSearchParams = serde_json::from_value(arguments)
                .map_err(|e| WebSearchError::InvalidArguments(e.to_string()))?;

            let query = SearchQuery {
                q: &params.query,
                count: params.max_results,
            };

            let mut request = self.client.get(&self.base_url).query(&query);

            if let Some(ref api_key) = self.api_key {
                request = request.bearer_auth(api_key);
            }

            let response = request.send().await?;

            let status = response.status();
            if !status.is_success() {
                let body = response.text().await.unwrap_or_default();
                let truncated = &body[..body.len().min(500)];
                return Ok(serde_json::json!({
                    "status": "error",
                    "message": format!("HTTP {status}: {truncated}"),
                }));
            }

            let text = response.text().await?;
            let results: Value = serde_json::from_str(&text)
                .unwrap_or_else(|_| serde_json::json!({ "content": text }));

            Ok(results)
        })
    }
}

fn default_max_results() -> usize {
    DEFAULT_MAX_RESULTS
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[derive(Debug)]
    struct StubRuntime;

    #[test]
    fn spec_declares_required_query_parameter() {
        let tool = WebSearchTool::<StubRuntime>::new("https://example.com/search");
        let spec = tool.spec();
        assert_eq!(spec.name, "web_search");
        let schema = &spec.input_schema;
        let required = schema["required"].as_array().unwrap();
        assert!(required.iter().any(|r| r == "query"));
    }

    #[tokio::test]
    async fn call_returns_error_on_invalid_arguments() {
        let tool = WebSearchTool::<StubRuntime>::new("https://example.com/search");
        let result = tool.call(&StubRuntime, json!("not an object")).await;
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error, WebSearchError::InvalidArguments(_)));
    }
}
