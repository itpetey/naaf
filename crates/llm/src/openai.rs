use std::{marker::PhantomData, sync::Arc};

use futures::future::LocalBoxFuture;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{Map, Value};
use thiserror::Error;

use crate::client::LlmClient;
use crate::message::{
    AssistantMessage, CompletionRequest, CompletionResponse, Message, ToolCall, ToolChoice, Usage,
};

const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";

pub trait OpenAiStreamObserver<R>: Send + Sync {
    fn on_reasoning_delta(&self, _runtime: &R, _delta: &str) {}

    fn on_response_complete(&self, _runtime: &R, _message: &AssistantMessage) {}
}

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
    stream_observer: Option<Arc<dyn OpenAiStreamObserver<R>>>,
    _marker: PhantomData<R>,
}

impl<R> OpenAiClient<R> {
    pub fn new(config: OpenAiConfig) -> Self {
        Self {
            config,
            http: Client::new(),
            stream_observer: None,
            _marker: PhantomData,
        }
    }

    pub fn from_env() -> Result<Self, OpenAiError> {
        Ok(Self::new(OpenAiConfig::from_env()?))
    }

    pub fn config(&self) -> &OpenAiConfig {
        &self.config
    }

    pub fn with_stream_observer(mut self, observer: Arc<dyn OpenAiStreamObserver<R>>) -> Self {
        self.stream_observer = Some(observer);
        self
    }
}

impl<R> LlmClient for OpenAiClient<R> {
    type Runtime = R;
    type Error = OpenAiError;

    fn complete<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        request: CompletionRequest,
    ) -> LocalBoxFuture<'a, Result<CompletionResponse, Self::Error>> {
        Box::pin(async move {
            let body = build_request_body(&request)?;
            let body = if self.stream_observer.is_some() {
                with_streaming_enabled(body)
            } else {
                body
            };

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

            if let Some(observer) = self.stream_observer.as_ref() {
                let content_type = response
                    .headers()
                    .get(reqwest::header::CONTENT_TYPE)
                    .and_then(|value| value.to_str().ok())
                    .unwrap_or_default()
                    .to_string();

                if content_type.starts_with("text/event-stream") {
                    return parse_stream_response(response, observer.as_ref(), runtime).await;
                }

                let api_response = response.json::<ApiResponse>().await?;
                let response = convert_response(api_response)?;
                emit_non_streaming_response(observer.as_ref(), runtime, &response);
                return Ok(response);
            }

            let api_response = response.json::<ApiResponse>().await?;
            convert_response(api_response)
        })
    }
}

fn with_streaming_enabled(mut body: Value) -> Value {
    if let Value::Object(map) = &mut body {
        map.insert("stream".to_string(), Value::Bool(true));
    }
    body
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
    let metadata = build_message_metadata(&choice.message);

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
    Ok(response.with_metadata(metadata))
}

fn build_message_metadata(message: &ApiMessageResponse) -> Value {
    let mut metadata = Map::new();

    if let Some(reasoning_content) = message.reasoning_content.as_ref()
        && !reasoning_content.trim().is_empty()
    {
        metadata.insert(
            "reasoning_content".to_string(),
            Value::String(reasoning_content.clone()),
        );
    }

    if let Some(reasoning) = message.reasoning.as_ref()
        && !reasoning.trim().is_empty()
    {
        metadata.insert("reasoning".to_string(), Value::String(reasoning.clone()));
    }

    Value::Object(metadata)
}

fn emit_non_streaming_response<R>(
    observer: &dyn OpenAiStreamObserver<R>,
    runtime: &R,
    response: &CompletionResponse,
) {
    if let Some(reasoning) = response
        .metadata
        .get("reasoning_content")
        .or_else(|| response.metadata.get("reasoning"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        observer.on_reasoning_delta(runtime, reasoning);
    }

    observer.on_response_complete(runtime, &response.message);
}

async fn parse_stream_response<R>(
    mut response: reqwest::Response,
    observer: &dyn OpenAiStreamObserver<R>,
    runtime: &R,
) -> Result<CompletionResponse, OpenAiError> {
    let mut state = StreamState::default();
    let mut line_buffer = String::new();
    let mut data_lines = Vec::new();

    while let Some(chunk) = response.chunk().await? {
        let chunk_text = std::str::from_utf8(&chunk)
            .map_err(|error| OpenAiError::Conversion(format!("invalid stream utf-8: {error}")))?;

        for ch in chunk_text.chars() {
            if ch == '\n' {
                process_stream_line(
                    std::mem::take(&mut line_buffer),
                    &mut data_lines,
                    &mut state,
                    observer,
                    runtime,
                )?;
            } else {
                line_buffer.push(ch);
            }
        }
    }

    if !line_buffer.is_empty() {
        process_stream_line(line_buffer, &mut data_lines, &mut state, observer, runtime)?;
    }

    if !data_lines.is_empty() {
        process_stream_event_data(&data_lines.join("\n"), &mut state, observer, runtime)?;
    }

    let response = state.into_response()?;
    observer.on_response_complete(runtime, &response.message);
    Ok(response)
}

fn process_stream_line<R>(
    mut line: String,
    data_lines: &mut Vec<String>,
    state: &mut StreamState,
    observer: &dyn OpenAiStreamObserver<R>,
    runtime: &R,
) -> Result<(), OpenAiError> {
    if line.ends_with('\r') {
        line.pop();
    }

    if line.is_empty() {
        if !data_lines.is_empty() {
            process_stream_event_data(&data_lines.join("\n"), state, observer, runtime)?;
            data_lines.clear();
        }
        return Ok(());
    }

    if let Some(data) = line.strip_prefix("data:") {
        data_lines.push(data.trim_start().to_string());
    }

    Ok(())
}

fn process_stream_event_data<R>(
    data: &str,
    state: &mut StreamState,
    observer: &dyn OpenAiStreamObserver<R>,
    runtime: &R,
) -> Result<(), OpenAiError> {
    let data = data.trim();
    if data.is_empty() || data == "[DONE]" {
        return Ok(());
    }

    let chunk: ApiStreamChunk = serde_json::from_str(data)
        .map_err(|error| OpenAiError::Conversion(format!("invalid stream chunk: {error}")))?;
    state.apply_chunk(chunk, observer, runtime)
}

#[derive(Default)]
struct StreamState {
    content: String,
    reasoning: String,
    tool_calls: Vec<StreamToolCall>,
    usage: Option<Usage>,
}

impl StreamState {
    fn apply_chunk<R>(
        &mut self,
        chunk: ApiStreamChunk,
        observer: &dyn OpenAiStreamObserver<R>,
        runtime: &R,
    ) -> Result<(), OpenAiError> {
        if let Some(usage) = chunk.usage {
            self.usage = Some(Usage {
                input_tokens: usage.prompt_tokens,
                output_tokens: usage.completion_tokens,
            });
        }

        for choice in chunk.choices {
            if let Some(reasoning) = choice
                .delta
                .reasoning_content
                .or(choice.delta.reasoning)
                .filter(|value| !value.is_empty())
            {
                self.reasoning.push_str(&reasoning);
                observer.on_reasoning_delta(runtime, &reasoning);
            }

            if let Some(content) = choice.delta.content {
                self.content.push_str(&content);
            }

            if let Some(tool_calls) = choice.delta.tool_calls {
                for tool_call in tool_calls {
                    self.append_tool_call_delta(tool_call);
                }
            }
        }

        Ok(())
    }

    fn append_tool_call_delta(&mut self, delta: ApiToolCallDelta) {
        while self.tool_calls.len() <= delta.index {
            self.tool_calls.push(StreamToolCall::default());
        }

        let tool_call = &mut self.tool_calls[delta.index];
        if let Some(id) = delta.id {
            tool_call.id = Some(id);
        }

        if let Some(function) = delta.function {
            if let Some(name) = function.name {
                tool_call.name = name;
            }
            if let Some(arguments) = function.arguments {
                tool_call.arguments.push_str(&arguments);
            }
        }
    }

    fn into_response(self) -> Result<CompletionResponse, OpenAiError> {
        let tool_calls = self
            .tool_calls
            .into_iter()
            .enumerate()
            .map(|(index, tool_call)| {
                let call_id = tool_call.id.ok_or_else(|| {
                    OpenAiError::Conversion(format!(
                        "missing streamed tool call id at index {index}"
                    ))
                })?;
                let arguments = if tool_call.arguments.trim().is_empty() {
                    Value::Object(Map::new())
                } else {
                    serde_json::from_str(&tool_call.arguments).map_err(|error| {
                        OpenAiError::Conversion(format!(
                            "invalid streamed tool call arguments for '{}': {error}",
                            tool_call.name
                        ))
                    })?
                };

                Ok(ToolCall {
                    call_id,
                    tool_name: tool_call.name,
                    arguments,
                })
            })
            .collect::<Result<Vec<_>, OpenAiError>>()?;

        let mut response = CompletionResponse::new(AssistantMessage {
            content: (!self.content.is_empty()).then_some(self.content),
            tool_calls,
        });

        if let Some(usage) = self.usage {
            response = response.with_usage(usage);
        }

        let metadata = if self.reasoning.trim().is_empty() {
            Value::Object(Map::new())
        } else {
            Value::Object(Map::from_iter([(
                "reasoning_content".to_string(),
                Value::String(self.reasoning),
            )]))
        };

        Ok(response.with_metadata(metadata))
    }
}

#[derive(Default)]
struct StreamToolCall {
    id: Option<String>,
    name: String,
    arguments: String,
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
    reasoning_content: Option<String>,
    reasoning: Option<String>,
    tool_calls: Option<Vec<ApiToolCallResponse>>,
}

#[derive(Deserialize)]
struct ApiStreamChunk {
    choices: Vec<ApiStreamChoice>,
    usage: Option<ApiUsage>,
}

#[derive(Deserialize)]
struct ApiStreamChoice {
    delta: ApiStreamDelta,
}

#[derive(Default, Deserialize)]
struct ApiStreamDelta {
    content: Option<String>,
    reasoning_content: Option<String>,
    reasoning: Option<String>,
    tool_calls: Option<Vec<ApiToolCallDelta>>,
}

#[derive(Deserialize)]
struct ApiToolCallDelta {
    index: usize,
    id: Option<String>,
    function: Option<ApiFunctionDelta>,
}

#[derive(Deserialize)]
struct ApiFunctionDelta {
    name: Option<String>,
    arguments: Option<String>,
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
    use std::sync::Mutex;

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
                    reasoning_content: None,
                    reasoning: None,
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
                    reasoning_content: None,
                    reasoning: None,
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
                    reasoning_content: None,
                    reasoning: None,
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

    #[test]
    fn convert_response_preserves_reasoning_metadata() {
        let api_response = ApiResponse {
            choices: vec![ApiChoice {
                message: ApiMessageResponse {
                    content: Some("Hello!".to_string()),
                    reasoning_content: Some("Thinking through the answer".to_string()),
                    reasoning: None,
                    tool_calls: None,
                },
            }],
            usage: None,
        };

        let response = convert_response(api_response).unwrap();
        assert_eq!(
            response.metadata["reasoning_content"].as_str(),
            Some("Thinking through the answer")
        );
    }

    #[test]
    fn stream_events_accumulate_reasoning_and_tool_calls() {
        let observer = TestObserver::default();
        let mut state = StreamState::default();
        let first_chunk = json!({
            "choices": [{
                "delta": {
                    "reasoning_content": "First thought. ",
                    "tool_calls": [{
                        "index": 0,
                        "id": "call-1",
                        "function": {
                            "name": "add",
                            "arguments": "{"
                        }
                    }]
                }
            }]
        })
        .to_string();
        let second_chunk = json!({
            "choices": [{
                "delta": {
                    "reasoning_content": "Second thought.",
                    "content": "Done",
                    "tool_calls": [{
                        "index": 0,
                        "function": {
                            "arguments": "\"left\":2}"
                        }
                    }]
                }
            }]
        })
        .to_string();

        process_stream_event_data(&first_chunk, &mut state, &observer, &()).unwrap();
        process_stream_event_data(&second_chunk, &mut state, &observer, &()).unwrap();

        let response = state.into_response().unwrap();
        assert_eq!(
            observer.reasoning.lock().unwrap().as_slice(),
            ["First thought. ".to_string(), "Second thought.".to_string()]
        );
        assert_eq!(response.message.content.as_deref(), Some("Done"));
        assert_eq!(response.message.tool_calls.len(), 1);
        assert_eq!(response.message.tool_calls[0].tool_name, "add");
        assert_eq!(response.message.tool_calls[0].arguments, json!({"left": 2}));
        assert_eq!(
            response.metadata["reasoning_content"].as_str(),
            Some("First thought. Second thought.")
        );
    }

    #[derive(Default)]
    struct TestObserver {
        reasoning: Mutex<Vec<String>>,
    }

    impl OpenAiStreamObserver<()> for TestObserver {
        fn on_reasoning_delta(&self, _runtime: &(), delta: &str) {
            self.reasoning.lock().unwrap().push(delta.to_string());
        }
    }
}
