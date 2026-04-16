use std::marker::PhantomData;

use futures::future::LocalBoxFuture;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;

use crate::client::LlmClient;
use crate::message::{
    AssistantMessage, CompletionRequest, CompletionResponse, Message, ToolCall, ToolChoice, Usage,
};

const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";

#[derive(Clone, Debug)]
pub struct OpenAiConfig {
    api_key: String,
    base_url: String,
    organisation: Option<String>,
}

impl OpenAiConfig {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: DEFAULT_BASE_URL.to_string(),
            organisation: None,
        }
    }

    pub fn from_env() -> Result<Self, OpenAiError> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| OpenAiError::Config("OPENAI_API_KEY not set".to_string()))?;
        let mut config = Self::new(api_key);
        if let Ok(base_url) = std::env::var("OPENAI_BASE_URL") {
            config = config.with_base_url(base_url);
        }
        if let Ok(org) = std::env::var("OPENAI_ORG_ID") {
            config = config.with_organisation(org);
        }
        Ok(config)
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    pub fn with_organisation(mut self, organisation: impl Into<String>) -> Self {
        self.organisation = Some(organisation.into());
        self
    }

    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn organisation(&self) -> Option<&str> {
        self.organisation.as_deref()
    }
}

#[derive(Debug, Error)]
pub enum OpenAiError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("OpenAI API error: {message}")]
    Api {
        message: String,
        error_type: String,
        code: Option<String>,
    },
    #[error("configuration error: {0}")]
    Config(String),
    #[error("failed to convert response: {0}")]
    Conversion(String),
}

#[derive(Clone)]
pub struct OpenAiClient<R> {
    config: OpenAiConfig,
    http: Client,
    _marker: PhantomData<R>,
}

impl<R> OpenAiClient<R> {
    pub fn new(config: OpenAiConfig) -> Self {
        Self {
            config,
            http: Client::new(),
            _marker: PhantomData,
        }
    }

    pub fn from_env() -> Result<Self, OpenAiError> {
        Ok(Self::new(OpenAiConfig::from_env()?))
    }

    pub fn config(&self) -> &OpenAiConfig {
        &self.config
    }
}

impl<R> LlmClient for OpenAiClient<R> {
    type Runtime = R;
    type Error = OpenAiError;

    fn complete<'a>(
        &'a self,
        _runtime: &'a Self::Runtime,
        request: CompletionRequest,
    ) -> LocalBoxFuture<'a, Result<CompletionResponse, Self::Error>> {
        Box::pin(async move {
            let body = build_request_body(&request)?;

            let url = format!("{}/chat/completions", self.config.base_url);
            let mut builder = self
                .http
                .post(&url)
                .bearer_auth(&self.config.api_key)
                .json(&body);

            if let Some(ref org) = self.config.organisation {
                builder = builder.header("OpenAI-Organization", org.as_str());
            }

            let response = builder.send().await?;

            let status = response.status();
            if !status.is_success() {
                let text = response.text().await.unwrap_or_default();
                return match serde_json::from_str::<ApiErrorResponse>(&text) {
                    Ok(error_body) => Err(OpenAiError::Api {
                        message: error_body.error.message,
                        error_type: error_body.error.r#type,
                        code: error_body.error.code,
                    }),
                    Err(_) => Err(OpenAiError::Api {
                        message: format!(
                            "HTTP {status} with non-JSON body: {}",
                            &text[..text.len().min(200)]
                        ),
                        error_type: "http_error".to_string(),
                        code: None,
                    }),
                };
            }

            let api_response = response.json::<ApiResponse>().await?;
            convert_response(api_response)
        })
    }
}

fn build_request_body(request: &CompletionRequest) -> Result<Value, OpenAiError> {
    let messages: Vec<Value> = request
        .messages
        .iter()
        .map(convert_message_to_value)
        .collect::<Result<Vec<_>, _>>()?;

    let mut body = serde_json::json!({
        "model": &request.model,
        "messages": messages,
    });

    if !request.tools.is_empty() {
        let tools: Vec<Value> = request
            .tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.input_schema,
                    }
                })
            })
            .collect();
        body["tools"] = Value::Array(tools);
        body["tool_choice"] = convert_tool_choice_to_value(&request.tool_choice);
    }

    if let Value::Object(map) = &request.metadata
        && let Value::Object(body_map) = &mut body
    {
        for (key, value) in map {
            body_map.insert(key.clone(), value.clone());
        }
    }

    Ok(body)
}

fn convert_message_to_value(msg: &Message) -> Result<Value, OpenAiError> {
    Ok(match msg {
        Message::System { content } => serde_json::json!({
            "role": "system",
            "content": content,
        }),
        Message::User { content } => serde_json::json!({
            "role": "user",
            "content": content,
        }),
        Message::Assistant(a) => {
            let tool_calls: Vec<Value> = a
                .tool_calls
                .iter()
                .map(|tc| {
                    serde_json::json!({
                        "id": tc.call_id,
                        "type": "function",
                        "function": {
                            "name": tc.tool_name,
                            "arguments": tc.arguments.to_string(),
                        }
                    })
                })
                .collect();
            let mut v = serde_json::json!({
                "role": "assistant",
                "content": a.content,
            });
            if !tool_calls.is_empty() {
                v["tool_calls"] = Value::Array(tool_calls);
            }
            v
        }
        Message::Tool(result) => serde_json::json!({
            "role": "tool",
            "tool_call_id": result.call_id,
            "content": result.content.to_string(),
        }),
    })
}

fn convert_tool_choice_to_value(choice: &ToolChoice) -> Value {
    match choice {
        ToolChoice::Auto => Value::String("auto".to_string()),
        ToolChoice::None => Value::String("none".to_string()),
        ToolChoice::Required(name) => serde_json::json!({
            "type": "function",
            "function": {"name": name}
        }),
    }
}

fn convert_response(response: ApiResponse) -> Result<CompletionResponse, OpenAiError> {
    let choice = response
        .choices
        .into_iter()
        .next()
        .ok_or_else(|| OpenAiError::Conversion("no choices in response".to_string()))?;

    let tool_calls = match choice.message.tool_calls {
        Some(calls) => calls
            .into_iter()
            .map(|tc| {
                let arguments = serde_json::from_str(&tc.function.arguments).map_err(|e| {
                    OpenAiError::Conversion(format!(
                        "invalid tool call arguments for '{}': {e}",
                        tc.function.name
                    ))
                })?;
                Ok(ToolCall {
                    call_id: tc.id,
                    tool_name: tc.function.name,
                    arguments,
                })
            })
            .collect::<Result<Vec<_>, OpenAiError>>()?,
        None => Vec::new(),
    };

    let response_usage = response.usage.map(|u| Usage {
        input_tokens: u.prompt_tokens,
        output_tokens: u.completion_tokens,
    });

    let mut response = CompletionResponse::new(AssistantMessage {
        content: choice.message.content,
        tool_calls,
    });
    if let Some(usage) = response_usage {
        response = response.with_usage(usage);
    }
    Ok(response.with_metadata(Value::Null))
}

#[derive(Deserialize)]
struct ApiResponse {
    choices: Vec<ApiChoice>,
    usage: Option<ApiUsage>,
}

#[derive(Deserialize)]
struct ApiChoice {
    message: ApiMessageResponse,
}

#[derive(Deserialize)]
struct ApiMessageResponse {
    content: Option<String>,
    tool_calls: Option<Vec<ApiToolCallResponse>>,
}

#[derive(Deserialize)]
struct ApiToolCallResponse {
    id: String,
    function: ApiFunctionResponse,
}

#[derive(Deserialize)]
struct ApiFunctionResponse {
    name: String,
    arguments: String,
}

#[derive(Deserialize)]
struct ApiUsage {
    prompt_tokens: u64,
    completion_tokens: u64,
}

#[derive(Deserialize)]
struct ApiErrorResponse {
    error: ApiErrorDetail,
}

#[derive(Deserialize)]
struct ApiErrorDetail {
    message: String,
    r#type: String,
    code: Option<String>,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::message::{
        AssistantMessage, Message, ToolCall, ToolChoice, ToolResultMessage, ToolSpec,
    };

    #[test]
    fn build_request_body_converts_system_message() {
        let request = CompletionRequest::new("gpt-4", vec![Message::system("You are helpful")]);
        let body = build_request_body(&request).unwrap();
        let messages = body["messages"].as_array().unwrap();
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[0]["content"], "You are helpful");
    }

    #[test]
    fn build_request_body_converts_user_message() {
        let request = CompletionRequest::new("gpt-4", vec![Message::user("Hello")]);
        let body = build_request_body(&request).unwrap();
        let messages = body["messages"].as_array().unwrap();
        assert_eq!(messages[0]["role"], "user");
        assert_eq!(messages[0]["content"], "Hello");
    }

    #[test]
    fn build_request_body_converts_assistant_with_tool_calls() {
        let request = CompletionRequest::new(
            "gpt-4",
            vec![Message::assistant(AssistantMessage::with_tool_calls(
                Some("Thinking".to_string()),
                vec![ToolCall {
                    call_id: "call-1".to_string(),
                    tool_name: "add".to_string(),
                    arguments: json!({"left": 2, "right": 3}),
                }],
            ))],
        );
        let body = build_request_body(&request).unwrap();
        let messages = body["messages"].as_array().unwrap();
        assert_eq!(messages[0]["role"], "assistant");
        assert_eq!(messages[0]["content"], "Thinking");
        let tool_calls = messages[0]["tool_calls"].as_array().unwrap();
        assert_eq!(tool_calls[0]["id"], "call-1");
        assert_eq!(tool_calls[0]["type"], "function");
        assert_eq!(tool_calls[0]["function"]["name"], "add");
        assert_eq!(
            tool_calls[0]["function"]["arguments"],
            r#"{"left":2,"right":3}"#
        );
    }

    #[test]
    fn build_request_body_converts_tool_result() {
        let request = CompletionRequest::new(
            "gpt-4",
            vec![Message::tool(ToolResultMessage {
                call_id: "call-1".to_string(),
                tool_name: "add".to_string(),
                content: json!({"sum": 5}),
            })],
        );
        let body = build_request_body(&request).unwrap();
        let messages = body["messages"].as_array().unwrap();
        assert_eq!(messages[0]["role"], "tool");
        assert_eq!(messages[0]["tool_call_id"], "call-1");
        assert_eq!(messages[0]["content"], r#"{"sum":5}"#);
    }

    #[test]
    fn build_request_body_converts_tools() {
        let request =
            CompletionRequest::new("gpt-4", vec![Message::user("Hello")]).with_tools(vec![
                ToolSpec {
                    name: "add".to_string(),
                    description: "Adds two numbers".to_string(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "left": {"type": "integer"},
                            "right": {"type": "integer"}
                        }
                    }),
                },
            ]);
        let body = build_request_body(&request).unwrap();
        let tools = body["tools"].as_array().unwrap();
        assert_eq!(tools[0]["type"], "function");
        assert_eq!(tools[0]["function"]["name"], "add");
        assert_eq!(tools[0]["function"]["description"], "Adds two numbers");
        assert_eq!(body["tool_choice"], "auto");
    }

    #[test]
    fn build_request_body_converts_tool_choice_required() {
        let request = CompletionRequest::new("gpt-4", vec![Message::user("Hello")])
            .with_tools(vec![ToolSpec {
                name: "add".to_string(),
                description: "Adds two numbers".to_string(),
                input_schema: json!({"type": "object"}),
            }])
            .with_tool_choice(ToolChoice::Required("add".to_string()));
        let body = build_request_body(&request).unwrap();
        assert_eq!(body["tool_choice"]["type"], "function");
        assert_eq!(body["tool_choice"]["function"]["name"], "add");
    }

    #[test]
    fn build_request_body_merges_metadata() {
        let request =
            CompletionRequest::new("gpt-4", vec![Message::user("Hello")]).with_metadata(json!({
                "temperature": 0.7,
                "max_tokens": 100
            }));
        let body = build_request_body(&request).unwrap();
        assert_eq!(body["temperature"], 0.7);
        assert_eq!(body["max_tokens"], 100);
    }

    #[test]
    fn convert_response_parses_simple_completion() {
        let api_response = ApiResponse {
            choices: vec![ApiChoice {
                message: ApiMessageResponse {
                    content: Some("Hello!".to_string()),
                    tool_calls: None,
                },
            }],
            usage: Some(ApiUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
            }),
        };
        let response = convert_response(api_response).unwrap();
        assert_eq!(response.message.content, Some("Hello!".to_string()));
        assert!(response.message.tool_calls.is_empty());
        let usage = response.usage.unwrap();
        assert_eq!(usage.input_tokens, 10);
        assert_eq!(usage.output_tokens, 5);
    }

    #[test]
    fn convert_response_parses_tool_calls() {
        let api_response = ApiResponse {
            choices: vec![ApiChoice {
                message: ApiMessageResponse {
                    content: None,
                    tool_calls: Some(vec![ApiToolCallResponse {
                        id: "call-1".to_string(),
                        function: ApiFunctionResponse {
                            name: "add".to_string(),
                            arguments: r#"{"left":2,"right":3}"#.to_string(),
                        },
                    }]),
                },
            }],
            usage: None,
        };
        let response = convert_response(api_response).unwrap();
        assert_eq!(response.message.content, None);
        assert_eq!(response.message.tool_calls.len(), 1);
        assert_eq!(response.message.tool_calls[0].call_id, "call-1");
        assert_eq!(response.message.tool_calls[0].tool_name, "add");
        assert_eq!(
            response.message.tool_calls[0].arguments,
            json!({"left":2,"right":3})
        );
    }

    #[test]
    fn convert_response_rejects_empty_choices() {
        let api_response = ApiResponse {
            choices: vec![],
            usage: None,
        };
        let result = convert_response(api_response);
        assert!(matches!(result, Err(OpenAiError::Conversion(_))));
    }

    #[test]
    fn convert_response_rejects_invalid_tool_arguments() {
        let api_response = ApiResponse {
            choices: vec![ApiChoice {
                message: ApiMessageResponse {
                    content: None,
                    tool_calls: Some(vec![ApiToolCallResponse {
                        id: "call-1".to_string(),
                        function: ApiFunctionResponse {
                            name: "add".to_string(),
                            arguments: "not json".to_string(),
                        },
                    }]),
                },
            }],
            usage: None,
        };
        let result = convert_response(api_response);
        assert!(matches!(result, Err(OpenAiError::Conversion(_))));
    }
}
