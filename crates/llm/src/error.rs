use thiserror::Error;

use crate::tool::ToolCallError;

/// Errors raised by the generic `naaf_core::Check` adapter.
pub type CheckError<BuildError, ClientError, ToolError, DecodeError> =
    AdapterError<BuildError, ClientError, ToolError, DecodeError>;
/// Errors raised by the generic `naaf_core::Materialiser` adapter.
pub type MaterialiserError<BuildError, ClientError, ToolError, DecodeError> =
    AdapterError<BuildError, ClientError, ToolError, DecodeError>;
/// Errors raised by the generic `naaf_core::RepairPlanner` adapter.
pub type RepairPlannerError<BuildError, ClientError, ToolError, DecodeError> =
    AdapterError<BuildError, ClientError, ToolError, DecodeError>;
/// Errors raised by the generic `naaf_core::Task` adapter.
pub type TaskError<BuildError, ClientError, ToolError, DecodeError> =
    AdapterError<BuildError, ClientError, ToolError, DecodeError>;

/// Errors raised while executing an LLM conversation.
#[derive(Debug, Error)]
pub enum ExecutorError<ClientError, ToolError> {
    /// The underlying LLM client failed.
    #[error("LLM client failed: {0}")]
    Client(#[source] ClientError),
    /// Tool dispatch or execution failed.
    #[error(transparent)]
    Tool(#[from] ToolCallError<ToolError>),
    /// The model never produced a final answer before the turn budget ran out.
    #[error("LLM execution exceeded the turn limit ({max_turns})")]
    TurnLimitExceeded {
        /// Maximum number of model turns permitted for the execution.
        max_turns: usize,
    },
}

/// Errors raised by the generic `naaf_core` role adapters.
#[derive(Debug, Error)]
pub enum AdapterError<BuildError, ClientError, ToolError, DecodeError> {
    /// Building the initial completion request failed.
    #[error("failed to build completion request: {0}")]
    Build(#[source] BuildError),
    /// LLM execution failed.
    #[error(transparent)]
    Execute(#[from] ExecutorError<ClientError, ToolError>),
    /// Decoding the completed conversation into the caller's output failed.
    #[error("failed to decode completion output: {0}")]
    Decode(#[source] DecodeError),
}
