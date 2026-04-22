use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A tool exposed to the model.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolSpec {
    /// Unique tool name passed back in tool calls.
    pub name: String,
    /// Human-readable description for the model.
    pub description: String,
    /// JSON Schema describing the accepted arguments.
    pub input_schema: Value,
}

/// A tool call emitted by the assistant.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    /// Provider-generated call identifier.
    pub call_id: String,
    /// Registered tool name to execute.
    pub tool_name: String,
    /// JSON arguments supplied by the model.
    pub arguments: Value,
}

/// A tool result appended back into the conversation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolResultMessage {
    /// The originating tool call identifier.
    pub call_id: String,
    /// The tool that produced this result.
    pub tool_name: String,
    /// Arbitrary JSON result content.
    pub content: Value,
}

/// The assistant message returned for one model turn.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AssistantMessage {
    /// Optional assistant text content.
    pub content: Option<String>,
    /// Requested tool calls to execute before the next turn.
    pub tool_calls: Vec<ToolCall>,
}

impl AssistantMessage {
    /// Creates an assistant message with text content and no tool calls.
    pub fn from_text(content: impl Into<String>) -> Self {
        Self {
            content: Some(content.into()),
            tool_calls: Vec::new(),
        }
    }

    /// Creates an assistant message containing tool calls and optional text.
    pub fn with_tool_calls(content: Option<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            content,
            tool_calls,
        }
    }
}

/// A provider-neutral conversation message.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Message {
    /// A system instruction.
    System {
        /// Raw instruction content sent as the system prompt.
        content: String,
    },
    /// A user message.
    User {
        /// Raw user-authored content.
        content: String,
    },
    /// A prior assistant response.
    Assistant(AssistantMessage),
    /// A tool result produced by the executor.
    Tool(ToolResultMessage),
}

impl Message {
    /// Creates a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self::System {
            content: content.into(),
        }
    }

    /// Creates a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self::User {
            content: content.into(),
        }
    }

    /// Creates an assistant message.
    pub fn assistant(message: AssistantMessage) -> Self {
        Self::Assistant(message)
    }

    /// Creates a tool message.
    pub fn tool(message: ToolResultMessage) -> Self {
        Self::Tool(message)
    }
}

/// Tool selection behaviour requested from the provider.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolChoice {
    /// Let the model decide whether to call tools.
    #[default]
    Auto,
    /// Disable tool calls for this request.
    None,
    /// Require the model to call a specific tool.
    Required(String),
}

/// Provider token usage accounting for one response.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    /// Input tokens sent to the provider.
    pub input_tokens: u64,
    /// Output tokens returned by the provider.
    pub output_tokens: u64,
}

/// One provider-neutral completion request.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompletionRequest {
    /// The model identifier to execute.
    pub model: String,
    /// Conversation history sent to the model.
    pub messages: Vec<Message>,
    /// Tools available to this request.
    pub tools: Vec<ToolSpec>,
    /// Provider-neutral tool selection policy.
    pub tool_choice: ToolChoice,
    /// Opaque provider-specific request metadata.
    pub metadata: Value,
}

impl CompletionRequest {
    /// Creates a request with no tools and empty metadata.
    pub fn new(model: impl Into<String>, messages: Vec<Message>) -> Self {
        Self {
            model: model.into(),
            messages,
            tools: Vec::new(),
            tool_choice: ToolChoice::Auto,
            metadata: Value::Null,
        }
    }

    /// Replaces the request tools.
    pub fn with_tools(mut self, tools: Vec<ToolSpec>) -> Self {
        self.tools = tools;
        self
    }

    /// Sets the tool selection behaviour.
    pub fn with_tool_choice(mut self, tool_choice: ToolChoice) -> Self {
        self.tool_choice = tool_choice;
        self
    }

    /// Sets provider-specific metadata.
    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = metadata;
        self
    }
}

/// One provider-neutral completion response.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompletionResponse {
    /// Assistant message produced for the turn.
    pub message: AssistantMessage,
    /// Optional provider token usage.
    pub usage: Option<Usage>,
    /// Opaque provider-specific response metadata.
    pub metadata: Value,
}

impl CompletionResponse {
    /// Creates a response with empty metadata and no usage accounting.
    pub fn new(message: AssistantMessage) -> Self {
        Self {
            message,
            usage: None,
            metadata: Value::Null,
        }
    }

    /// Sets token usage for the response.
    pub fn with_usage(mut self, usage: Usage) -> Self {
        self.usage = Some(usage);
        self
    }

    /// Sets provider-specific metadata.
    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = metadata;
        self
    }
}
