use std::convert::Infallible;
use std::marker::PhantomData;

use futures::future::LocalBoxFuture;
use naaf_core::Task;

use crate::{
    client::LlmClient,
    error::TaskError,
    executor::{ExecutionOutcome, Executor},
    message::CompletionRequest,
};

type TaskMarker<Input, Output, BuildError, DecodeError> =
    PhantomData<fn(Input) -> Result<Output, (BuildError, DecodeError)>>;

/// A generic `naaf_core::Task` backed by an LLM executor.
pub struct LlmTask<
    C,
    R,
    Build,
    Decode,
    Input,
    Output,
    BuildError,
    DecodeError,
    ToolError = Infallible,
> {
    executor: Executor<C, R, ToolError>,
    build_request: Build,
    decode_output: Decode,
    marker: TaskMarker<Input, Output, BuildError, DecodeError>,
}

impl<C, R, Build, Decode, Input, Output, BuildError, DecodeError>
    LlmTask<C, R, Build, Decode, Input, Output, BuildError, DecodeError, Infallible>
{
    /// Creates an LLM task without tools.
    pub fn new(
        executor: Executor<C, R, Infallible>,
        build_request: Build,
        decode_output: Decode,
    ) -> Self {
        Self {
            executor,
            build_request,
            decode_output,
            marker: PhantomData,
        }
    }
}

impl<C, R, Build, Decode, Input, Output, BuildError, DecodeError, ToolError>
    LlmTask<C, R, Build, Decode, Input, Output, BuildError, DecodeError, ToolError>
{
    /// Creates an LLM task with a preconfigured executor.
    pub fn with_executor(
        executor: Executor<C, R, ToolError>,
        build_request: Build,
        decode_output: Decode,
    ) -> Self {
        Self {
            executor,
            build_request,
            decode_output,
            marker: PhantomData,
        }
    }

    /// Returns the underlying executor.
    pub fn executor(&self) -> &Executor<C, R, ToolError> {
        &self.executor
    }
}

impl<C, R, Build, Decode, Input, Output, BuildError, DecodeError, ToolError> Task
    for LlmTask<C, R, Build, Decode, Input, Output, BuildError, DecodeError, ToolError>
where
    C: LlmClient<Runtime = R> + 'static,
    C::Error: 'static,
    R: 'static,
    Input: 'static,
    Output: 'static,
    BuildError: 'static,
    DecodeError: 'static,
    ToolError: 'static,
    Build: Fn(&R, Input) -> Result<CompletionRequest, BuildError> + 'static,
    Decode: Fn(ExecutionOutcome) -> Result<Output, DecodeError> + 'static,
{
    type Runtime = R;
    type Input = Input;
    type Output = Output;
    type Error = TaskError<BuildError, C::Error, ToolError, DecodeError>;

    fn run<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        input: Self::Input,
    ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
        Box::pin(async move {
            let request = (self.build_request)(runtime, input).map_err(TaskError::Build)?;
            let outcome = self
                .executor
                .execute(runtime, request)
                .await
                .map_err(TaskError::Execute)?;
            (self.decode_output)(outcome).map_err(TaskError::Decode)
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{
        cell::RefCell,
        convert::Infallible,
        fmt::{Display, Formatter},
    };

    use futures::future::LocalBoxFuture;
    use naaf_core::Task;
    use serde_json::{Value, json};

    use super::LlmTask;
    use crate::{
        AssistantMessage, CompletionRequest, CompletionResponse, ExecutionOutcome, Executor,
        LlmClient, Message, Tool, ToolCall, ToolRegistry, ToolSpec,
    };

    #[derive(Debug, Default)]
    struct TestRuntime;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TestError(&'static str);

    impl Display for TestError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            f.write_str(self.0)
        }
    }

    impl std::error::Error for TestError {}

    #[derive(Debug)]
    struct StubClient {
        requests: RefCell<Vec<CompletionRequest>>,
        responses: RefCell<Vec<CompletionResponse>>,
    }

    impl StubClient {
        fn new(responses: Vec<CompletionResponse>) -> Self {
            Self {
                requests: RefCell::new(Vec::new()),
                responses: RefCell::new(responses.into_iter().rev().collect()),
            }
        }

        fn requests(&self) -> Vec<CompletionRequest> {
            self.requests.borrow().clone()
        }
    }

    impl LlmClient for StubClient {
        type Runtime = TestRuntime;
        type Error = TestError;

        fn complete<'a>(
            &'a self,
            _runtime: &'a Self::Runtime,
            request: CompletionRequest,
        ) -> LocalBoxFuture<'a, Result<CompletionResponse, Self::Error>> {
            self.requests.borrow_mut().push(request);
            Box::pin(async move {
                self.responses
                    .borrow_mut()
                    .pop()
                    .ok_or(TestError("missing stub response"))
            })
        }
    }

    struct AddTool;

    impl Tool for AddTool {
        type Runtime = TestRuntime;
        type Error = TestError;

        fn spec(&self) -> ToolSpec {
            ToolSpec {
                name: "add".to_string(),
                description: "Adds two integers".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "left": { "type": "integer" },
                        "right": { "type": "integer" }
                    },
                    "required": ["left", "right"]
                }),
            }
        }

        fn call<'a>(
            &'a self,
            _runtime: &'a Self::Runtime,
            arguments: Value,
        ) -> LocalBoxFuture<'a, Result<Value, Self::Error>> {
            Box::pin(async move {
                let left = arguments
                    .get("left")
                    .and_then(Value::as_i64)
                    .ok_or(TestError("missing left"))?;
                let right = arguments
                    .get("right")
                    .and_then(Value::as_i64)
                    .ok_or(TestError("missing right"))?;
                Ok(json!({ "sum": left + right }))
            })
        }
    }

    #[tokio::test]
    async fn llm_task_decodes_plain_completion() {
        let client = StubClient::new(vec![CompletionResponse::new(AssistantMessage::from_text(
            "final answer",
        ))]);
        let task = LlmTask::new(
            Executor::new(client),
            |_runtime: &TestRuntime, input: String| {
                Ok::<_, Infallible>(CompletionRequest::new(
                    "test-model",
                    vec![Message::user(input)],
                ))
            },
            |outcome: ExecutionOutcome| {
                Ok::<_, Infallible>(outcome.final_message().content.clone().unwrap_or_default())
            },
        );

        let output = task
            .run(&TestRuntime, "hello".to_string())
            .await
            .expect("plain completion should succeed");

        assert_eq!(output, "final answer");
    }

    #[tokio::test]
    async fn llm_task_executes_tool_calls_until_completion() {
        let client = StubClient::new(vec![
            CompletionResponse::new(AssistantMessage::with_tool_calls(
                Some("Calculating".to_string()),
                vec![ToolCall {
                    call_id: "call-1".to_string(),
                    tool_name: "add".to_string(),
                    arguments: json!({ "left": 2, "right": 3 }),
                }],
            )),
            CompletionResponse::new(AssistantMessage::from_text("The total is 5")),
        ]);
        let mut tools = ToolRegistry::new();
        tools.register(AddTool).expect("tool should register");

        let task = LlmTask::with_executor(
            Executor::with_tools(client, tools),
            |_runtime: &TestRuntime, input: String| {
                Ok::<_, Infallible>(CompletionRequest::new(
                    "test-model",
                    vec![Message::system("Be concise"), Message::user(input)],
                ))
            },
            |outcome: ExecutionOutcome| {
                let requests = outcome.responses().len();
                let saw_tool_result = outcome
                    .messages()
                    .iter()
                    .any(|message| matches!(message, Message::Tool(_)));
                Ok::<_, Infallible>((
                    outcome.final_message().content.clone().unwrap_or_default(),
                    requests,
                    saw_tool_result,
                ))
            },
        );

        let output = task
            .run(&TestRuntime, "what is 2 + 3?".to_string())
            .await
            .expect("tool-calling completion should succeed");

        assert_eq!(output.0, "The total is 5");
        assert_eq!(output.1, 2);
        assert!(output.2);
        assert_eq!(task.executor().client().requests().len(), 2);
        assert_eq!(task.executor().tools().specs().len(), 1);
    }
}
