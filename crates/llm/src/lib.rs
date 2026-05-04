//! LLM-backed `naaf_core` role infrastructure.
#![warn(missing_docs)]
//!
//! `naaf-llm` keeps the outer workflow contract in `naaf-core` while handling
//! the inner model execution loop, including tool calls. Callers stay in
//! control of request construction and output decoding so the crate does not
//! impose a prompt or response format.
//!
//! A shared [`LlmAgent`] can then be projected into typed `Task`, `Check`,
//! `Materialiser`, and `RepairPlanner` implementations without duplicating the
//! provider wiring.
//!
//! # Example
//!
//! ```
//! use std::{cell::RefCell, convert::Infallible, fmt::{Display, Formatter}};
//!
//! use futures::{FutureExt, future::LocalBoxFuture};
//! use naaf_core::{Check, Step, Task};
//! use naaf_llm::{
//!     AssistantMessage, CompletionRequest, CompletionResponse, ExecutionOutcome, Executor,
//!     LlmAgent, LlmClient, Message, TaskError, Tool, ToolCall, ToolRegistry, ToolSpec,
//! };
//! use serde_json::{Value, json};
//!
//! #[derive(Debug, Default)]
//! struct Runtime;
//!
//! #[derive(Debug, Clone, PartialEq, Eq)]
//! struct Error(&'static str);
//!
//! impl Display for Error {
//!     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//!         f.write_str(self.0)
//!     }
//! }
//!
//! impl std::error::Error for Error {}
//!
//! #[derive(Debug)]
//! struct StubClient {
//!     responses: RefCell<Vec<CompletionResponse>>,
//! }
//!
//! impl StubClient {
//!     fn new(responses: Vec<CompletionResponse>) -> Self {
//!         Self {
//!             responses: RefCell::new(responses.into_iter().rev().collect()),
//!         }
//!     }
//! }
//!
//! impl LlmClient for StubClient {
//!     type Runtime = Runtime;
//!     type Error = Error;
//!
//!     fn complete<'a>(
//!         &'a self,
//!         _runtime: &'a Self::Runtime,
//!         _request: CompletionRequest,
//!     ) -> LocalBoxFuture<'a, Result<CompletionResponse, Self::Error>> {
//!         Box::pin(async move {
//!             self.responses
//!                 .borrow_mut()
//!                 .pop()
//!                 .ok_or(Error("missing stub response"))
//!         })
//!     }
//! }
//!
//! struct AddTool;
//!
//! impl Tool for AddTool {
//!     type Runtime = Runtime;
//!     type Error = Error;
//!
//!     fn spec(&self) -> ToolSpec {
//!         ToolSpec {
//!             name: "add".to_string(),
//!             description: "Adds two integers".to_string(),
//!             input_schema: json!({
//!                 "type": "object",
//!                 "properties": {
//!                     "left": { "type": "integer" },
//!                     "right": { "type": "integer" }
//!                 },
//!                 "required": ["left", "right"]
//!             }),
//!         }
//!     }
//!
//!     fn call<'a>(
//!         &'a self,
//!         _runtime: &'a Self::Runtime,
//!         arguments: Value,
//!     ) -> LocalBoxFuture<'a, Result<Value, Self::Error>> {
//!         Box::pin(async move {
//!             let left = arguments
//!                 .get("left")
//!                 .and_then(Value::as_i64)
//!                 .ok_or(Error("missing left"))?;
//!             let right = arguments
//!                 .get("right")
//!                 .and_then(Value::as_i64)
//!                 .ok_or(Error("missing right"))?;
//!             Ok(json!({ "sum": left + right }))
//!         })
//!     }
//! }
//!
//! struct MentionsTotal;
//!
//! impl Check for MentionsTotal {
//!     type Runtime = Runtime;
//!     type Input = String;
//!     type Output = String;
//!     type Finding = &'static str;
//!     type Error = TaskError<Infallible, Error, Error, Infallible>;
//!
//!     fn check<'a>(
//!         &'a self,
//!         _runtime: &'a Self::Runtime,
//!         _input: Self::Input,
//!         output: Self::Output,
//!     ) -> LocalBoxFuture<'a, Result<Vec<Self::Finding>, Self::Error>> {
//!         Box::pin(async move {
//!             if output.contains('5') {
//!                 Ok(Vec::new())
//!             } else {
//!                 Ok(vec!["answer did not mention the calculated total"])
//!             }
//!         })
//!     }
//! }
//!
//! let client = StubClient::new(vec![
//!     CompletionResponse::new(AssistantMessage::with_tool_calls(
//!         Some("Calculating".to_string()),
//!         vec![ToolCall {
//!             call_id: "call-1".to_string(),
//!             tool_name: "add".to_string(),
//!             arguments: json!({ "left": 2, "right": 3 }),
//!         }],
//!     )),
//!     CompletionResponse::new(AssistantMessage::from_text("The total is 5")),
//! ]);
//!
//! let tools = ToolRegistry::new()
//!     .with_tool(AddTool)
//!     .expect("tool registry should accept unique tool names");
//! let llm = LlmAgent::with_executor(Executor::with_tools(client, tools));
//! let task = llm.task(
//!     |_runtime: &Runtime, question: String| {
//!         Ok::<_, Infallible>(CompletionRequest::new(
//!             "stub-model",
//!             vec![Message::user(question)],
//!         ))
//!     },
//!     |outcome: ExecutionOutcome| {
//!         Ok::<_, Infallible>(
//!             outcome
//!                 .final_message()
//!                 .content
//!                 .clone()
//!                 .unwrap_or_default(),
//!         )
//!     },
//! );
//!
//! let step = Step::builder(task).validate(MentionsTotal).build();
//! let traced = step
//!     .run_traced(&Runtime, "What is 2 + 3?".to_string())
//!     .now_or_never()
//!     .expect("fake provider completes immediately")
//!     .expect("step should succeed");
//!
//! assert_eq!(traced.output(), "The total is 5");
//! assert_eq!(traced.report().attempt_count(), 1);
//! assert!(traced.report().attempts()[0].accepted());
//! ```
//!
//! # Structured Output Example
//!
//! ```
//! use std::{cell::RefCell, convert::Infallible, fmt::{Display, Formatter}};
//!
//! use futures::{FutureExt, future::LocalBoxFuture};
//! use naaf_core::{Check, Step, Task};
//! use naaf_llm::{
//!     AssistantMessage, CompletionRequest, CompletionResponse, ExecutionOutcome, Executor,
//!     LlmClient, LlmTask, Message, TaskError,
//! };
//! use serde::Deserialize;
//!
//! #[derive(Debug, Default)]
//! struct Runtime;
//!
//! #[derive(Debug, Clone, PartialEq, Eq)]
//! struct Error(&'static str);
//!
//! impl Display for Error {
//!     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//!         f.write_str(self.0)
//!     }
//! }
//!
//! impl std::error::Error for Error {}
//!
//! #[derive(Debug)]
//! struct StubClient {
//!     responses: RefCell<Vec<CompletionResponse>>,
//! }
//!
//! impl StubClient {
//!     fn new(responses: Vec<CompletionResponse>) -> Self {
//!         Self {
//!             responses: RefCell::new(responses.into_iter().rev().collect()),
//!         }
//!     }
//! }
//!
//! impl LlmClient for StubClient {
//!     type Runtime = Runtime;
//!     type Error = Error;
//!
//!     fn complete<'a>(
//!         &'a self,
//!         _runtime: &'a Self::Runtime,
//!         _request: CompletionRequest,
//!     ) -> LocalBoxFuture<'a, Result<CompletionResponse, Self::Error>> {
//!         Box::pin(async move {
//!             self.responses
//!                 .borrow_mut()
//!                 .pop()
//!                 .ok_or(Error("missing stub response"))
//!         })
//!     }
//! }
//!
//! #[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
//! struct Answer {
//!     total: i64,
//! }
//!
//! struct HasExpectedTotal;
//!
//! impl Check for HasExpectedTotal {
//!     type Runtime = Runtime;
//!     type Input = String;
//!     type Output = Answer;
//!     type Finding = &'static str;
//!     type Error = TaskError<Infallible, Error, Infallible, serde_json::Error>;
//!
//!     fn check<'a>(
//!         &'a self,
//!         _runtime: &'a Self::Runtime,
//!         _input: Self::Input,
//!         output: Self::Output,
//!     ) -> LocalBoxFuture<'a, Result<Vec<Self::Finding>, Self::Error>> {
//!         Box::pin(async move {
//!             if output.total == 5 {
//!                 Ok(Vec::new())
//!             } else {
//!                 Ok(vec!["decoded answer did not match the expected total"])
//!             }
//!         })
//!     }
//! }
//!
//! let client = StubClient::new(vec![CompletionResponse::new(AssistantMessage::from_text(
//!     r#"{"total":5}"#,
//! ))]);
//!
//! let task = LlmTask::new(
//!     Executor::new(client),
//!     |_runtime: &Runtime, question: String| {
//!         Ok::<_, Infallible>(CompletionRequest::new(
//!             "stub-model",
//!             vec![Message::user(question)],
//!         ))
//!     },
//!     |outcome: ExecutionOutcome| {
//!         serde_json::from_str(
//!             outcome
//!                 .final_message()
//!                 .content
//!                 .as_deref()
//!                 .unwrap_or("null"),
//!         )
//!     },
//! );
//!
//! let step = Step::builder(task).validate(HasExpectedTotal).build();
//! let traced = step
//!     .run_traced(&Runtime, "Return the answer as JSON".to_string())
//!     .now_or_never()
//!     .expect("fake provider completes immediately")
//!     .expect("step should succeed");
//!
//! assert_eq!(traced.output(), &Answer { total: 5 });
//! assert!(traced.report().attempts()[0].accepted());
//! ```

#[cfg(feature = "openai")]
pub use crate::openai::{OpenAiClient, OpenAiConfig, OpenAiError, OpenAiStreamObserver};
#[cfg(feature = "web-search")]
pub use crate::web_search::{WebSearchError, WebSearchParams, WebSearchTool};
pub use crate::{
    agent::LlmAgent,
    client::LlmClient,
    error::{
        AdaptorError, CheckError, ExecutorError, MaterialiserError, RepairPlannerError, TaskError,
    },
    executor::{ExecutionOutcome, Executor, ExecutorConfig},
    human::{
        ChannelHumanIO, HumanAnswer, HumanIO, HumanQuestion, PendingQuestion, QuestionTool,
        StdinError, StdinHumanIO,
    },
    message::{
        AssistantMessage, CompletionRequest, CompletionResponse, Message, ToolCall, ToolChoice,
        ToolResultMessage, ToolSpec, Usage,
    },
    message_source::MessageSource,
    repository::{GlobPathsTool, ReadFileTool, SearchFilesTool},
    task::{LlmCheck, LlmMaterialiser, LlmRepairPlanner, LlmTask},
    tool::{RegisterToolError, Tool, ToolCallError, ToolRegistry},
};

mod agent;
mod client;
mod error;
mod executor;
mod human;
mod message;
mod message_source;
#[cfg(feature = "openai")]
mod openai;
/// Repository-aware tools for reading and searching a workspace from an LLM.
pub mod repository;
mod task;
mod tool;
#[cfg(feature = "web-search")]
mod web_search;
