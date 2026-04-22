use std::{collections::BTreeMap, convert::Infallible, sync::Arc};

use futures::future::LocalBoxFuture;
use serde_json::Value;
use thiserror::Error;

use crate::message::{ToolCall, ToolResultMessage, ToolSpec};

type RegisteredTool<R, E> = Arc<dyn Tool<Runtime = R, Error = E>>;

/// Executes one named tool against JSON arguments.
pub trait Tool {
    /// Shared runtime capabilities used by this tool.
    type Runtime;
    /// Errors thrown by the tool itself.
    type Error;

    /// Returns the tool specification advertised to the model.
    fn spec(&self) -> ToolSpec;

    /// Executes the tool call arguments and returns JSON output.
    fn call<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        arguments: Value,
    ) -> LocalBoxFuture<'a, Result<Value, Self::Error>>;
}

/// A registry of tools exposed to the model.
#[derive(Clone)]
pub struct ToolRegistry<R, E = Infallible> {
    tools: BTreeMap<String, RegisteredTool<R, E>>,
}

/// Errors raised while building a registry.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum RegisterToolError {
    /// Two tools used the same public name.
    #[error("tool '{name}' is already registered")]
    DuplicateTool {
        /// The duplicated tool name.
        name: String,
    },
}

/// Errors raised while dispatching or executing a tool call.
#[derive(Debug, Error)]
pub enum ToolCallError<E> {
    /// The assistant requested a tool that was not registered.
    #[error("tool '{tool}' is not registered for call '{call_id}'")]
    UnknownTool {
        /// Name of the requested tool.
        tool: String,
        /// Provider call identifier for the failed invocation.
        call_id: String,
    },
    /// The selected tool returned an execution error.
    #[error("tool '{tool}' failed for call '{call_id}': {error}")]
    Execution {
        /// Name of the tool that failed.
        tool: String,
        /// Provider call identifier for the failed invocation.
        call_id: String,
        #[source]
        /// Underlying tool error.
        error: E,
    },
}

impl<R, E> Default for ToolRegistry<R, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R, E> ToolRegistry<R, E> {
    /// Creates an empty tool registry.
    pub fn new() -> Self {
        Self {
            tools: BTreeMap::new(),
        }
    }

    /// Returns whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Registers a tool by its declared name.
    pub fn register<T>(&mut self, tool: T) -> Result<(), RegisterToolError>
    where
        T: Tool<Runtime = R, Error = E> + 'static,
    {
        let spec = tool.spec();
        let name = spec.name;

        if self.tools.contains_key(&name) {
            return Err(RegisterToolError::DuplicateTool { name });
        }

        self.tools.insert(name, Arc::new(tool));
        Ok(())
    }

    /// Registers a tool and returns the registry for chaining.
    pub fn with_tool<T>(mut self, tool: T) -> Result<Self, RegisterToolError>
    where
        T: Tool<Runtime = R, Error = E> + 'static,
    {
        self.register(tool)?;
        Ok(self)
    }

    /// Returns the registered tool specifications in deterministic order.
    pub fn specs(&self) -> Vec<ToolSpec> {
        self.tools.values().map(|tool| tool.spec()).collect()
    }

    /// Executes the tool call and wraps the JSON output into a conversation message.
    pub fn execute<'a>(
        &'a self,
        runtime: &'a R,
        tool_call: ToolCall,
    ) -> LocalBoxFuture<'a, Result<ToolResultMessage, ToolCallError<E>>>
    where
        E: 'static,
    {
        Box::pin(async move {
            let Some(tool) = self.tools.get(&tool_call.tool_name).cloned() else {
                return Err(ToolCallError::UnknownTool {
                    tool: tool_call.tool_name,
                    call_id: tool_call.call_id,
                });
            };

            let ToolCall {
                call_id,
                tool_name,
                arguments,
            } = tool_call;

            let content =
                tool.call(runtime, arguments)
                    .await
                    .map_err(|error| ToolCallError::Execution {
                        tool: tool_name.clone(),
                        call_id: call_id.clone(),
                        error,
                    })?;

            Ok(ToolResultMessage {
                call_id,
                tool_name,
                content,
            })
        })
    }
}
