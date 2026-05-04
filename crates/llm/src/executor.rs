use std::{convert::Infallible, sync::Arc};

use futures::future::LocalBoxFuture;
use tracing::{debug, trace};

use crate::{
    client::LlmClient,
    error::ExecutorError,
    message::{AssistantMessage, CompletionRequest, CompletionResponse, Message},
    message_source::MessageSource,
    tool::ToolRegistry,
};

/// Configures the inner model/tool loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExecutorConfig {
    max_turns: usize,
    max_input_tokens: Option<usize>,
}

/// The completed conversation captured by the executor.
#[derive(Clone, Debug, PartialEq)]
pub struct ExecutionOutcome {
    messages: Vec<Message>,
    responses: Vec<CompletionResponse>,
}

/// Executes provider turns until a final assistant answer is produced.
#[derive(Clone)]
pub struct Executor<C, R, E = Infallible> {
    client: C,
    tools: ToolRegistry<R, E>,
    config: ExecutorConfig,
    message_source: Option<Arc<dyn MessageSource>>,
}

impl ExecutorConfig {
    /// Creates a config with the given maximum number of model turns.
    pub fn new(max_turns: usize) -> Self {
        assert!(max_turns > 0, "executor must allow at least one turn");
        Self {
            max_turns,
            max_input_tokens: None,
        }
    }

    /// Sets an optional maximum input token budget for the execution.
    ///
    /// When configured, the executor checks the accumulated message token count
    /// before each turn and returns [`ExecutorError::TokenLimitExceeded`] if
    /// the budget would be exceeded.
    pub fn with_max_input_tokens(mut self, max_input_tokens: usize) -> Self {
        self.max_input_tokens = Some(max_input_tokens);
        self
    }

    /// Returns the maximum number of model turns.
    pub fn max_turns(self) -> usize {
        self.max_turns
    }

    /// Returns the maximum input token budget, if configured.
    pub fn max_input_tokens(self) -> Option<usize> {
        self.max_input_tokens
    }
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self::new(8)
    }
}

impl ExecutionOutcome {
    /// Creates an execution outcome from the captured transcript and responses.
    pub fn new(messages: Vec<Message>, responses: Vec<CompletionResponse>) -> Self {
        Self {
            messages,
            responses,
        }
    }

    /// Returns the full transcript, including appended tool results.
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// Returns the raw provider responses.
    pub fn responses(&self) -> &[CompletionResponse] {
        &self.responses
    }

    /// Returns the final provider response.
    pub fn final_response(&self) -> &CompletionResponse {
        match self.responses.last() {
            Some(response) => response,
            None => unreachable!("execution outcomes always contain at least one response"),
        }
    }

    /// Returns the final assistant message.
    pub fn final_message(&self) -> &AssistantMessage {
        &self.final_response().message
    }

    /// Consumes the outcome and returns its transcript and responses.
    pub fn into_parts(self) -> (Vec<Message>, Vec<CompletionResponse>) {
        (self.messages, self.responses)
    }
}

impl<C, R> Executor<C, R, Infallible> {
    /// Creates an executor with no tools.
    pub fn new(client: C) -> Self {
        Self {
            client,
            tools: ToolRegistry::new(),
            config: ExecutorConfig::default(),
            message_source: None,
        }
    }
}

impl<C, R, E> Executor<C, R, E> {
    /// Creates an executor with the given tool registry.
    pub fn with_tools(client: C, tools: ToolRegistry<R, E>) -> Self {
        Self {
            client,
            tools,
            config: ExecutorConfig::default(),
            message_source: None,
        }
    }

    /// Replaces the executor config.
    pub fn with_config(mut self, config: ExecutorConfig) -> Self {
        self.config = config;
        self
    }

    /// Sets a message source that the executor will drain after each batch of
    /// tool calls, injecting queued user messages into the conversation before
    /// the next turn.
    pub fn with_message_source(mut self, source: Arc<dyn MessageSource>) -> Self {
        self.message_source = Some(source);
        self
    }

    /// Returns the configured tool registry.
    pub fn tools(&self) -> &ToolRegistry<R, E> {
        &self.tools
    }

    /// Returns the current executor config.
    pub fn config(&self) -> ExecutorConfig {
        self.config
    }

    /// Returns the inner client.
    pub fn client(&self) -> &C {
        &self.client
    }

    /// Returns the message source, if configured.
    pub fn message_source(&self) -> Option<&Arc<dyn MessageSource>> {
        self.message_source.as_ref()
    }
}

impl<C, R, E> Executor<C, R, E>
where
    C: LlmClient<Runtime = R>,
    C::Error: 'static,
    E: 'static,
{
    /// Executes the completion request until the assistant stops requesting tools.
    pub fn execute<'a>(
        &'a self,
        runtime: &'a R,
        request: CompletionRequest,
    ) -> LocalBoxFuture<'a, Result<ExecutionOutcome, ExecutorError<C::Error, E>>> {
        Box::pin(async move {
            let CompletionRequest {
                model,
                mut messages,
                tool_choice,
                metadata,
                ..
            } = request;
            let mut responses = Vec::new();
            let tools = self.tools.specs();

            for turn in 1..=self.config.max_turns() {
                if let Some(max_tokens) = self.config.max_input_tokens() {
                    let token_count = messages.iter().map(|m| m.token_count()).sum::<usize>();
                    if token_count > max_tokens {
                        return Err(ExecutorError::TokenLimitExceeded { max_tokens });
                    }
                }

                debug!(turn, model = %model, tool_count = tools.len(), "starting LLM turn");

                let response = self
                    .client
                    .complete(
                        runtime,
                        CompletionRequest {
                            model: model.clone(),
                            messages: messages.clone(),
                            tools: tools.clone(),
                            tool_choice: tool_choice.clone(),
                            metadata: metadata.clone(),
                        },
                    )
                    .await
                    .map_err(ExecutorError::Client)?;

                let assistant = response.message.clone();
                let tool_call_count = assistant.tool_calls.len();
                trace!(turn, tool_call_count, "assistant response received");

                messages.push(Message::assistant(assistant.clone()));
                responses.push(response);

                if assistant.tool_calls.is_empty() {
                    return Ok(ExecutionOutcome::new(messages, responses));
                }

                for tool_call in assistant.tool_calls {
                    debug!(turn, tool = %tool_call.tool_name, call_id = %tool_call.call_id, "executing tool call");
                    let result = self
                        .tools
                        .execute(runtime, tool_call)
                        .await
                        .map_err(ExecutorError::Tool)?;
                    messages.push(Message::tool(result));
                }

                if let Some(ref source) = self.message_source {
                    let queued = source.drain_messages();
                    for msg in queued {
                        trace!(turn, "injecting queued user message");
                        messages.push(Message::user(msg));
                    }
                }
            }

            Err(ExecutorError::TurnLimitExceeded {
                max_turns: self.config.max_turns(),
            })
        })
    }
}
