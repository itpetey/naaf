use std::{convert::Infallible, marker::PhantomData, sync::Arc};

use futures::future::LocalBoxFuture;
use naaf_core::{Attempt, Check, Materialiser, RepairPlanner, Task};

use crate::{
    client::LlmClient,
    error::AdaptorError,
    executor::{ExecutionOutcome, Executor},
    message::CompletionRequest,
};

type AdapterFuture<'a, C, Output, BuildError, ToolError, DecodeError> =
    LocalBoxFuture<'a, AdapterResult<C, Output, BuildError, ToolError, DecodeError>>;
type AdapterMarker<Input, Output, BuildError, DecodeError> =
    PhantomData<fn(Input) -> Result<Output, (BuildError, DecodeError)>>;
type AdapterResult<C, Output, BuildError, ToolError, DecodeError> =
    Result<Output, AdaptorError<BuildError, <C as LlmClient>::Error, ToolError, DecodeError>>;
type RepairAdapter<
    C,
    R,
    Build,
    Decode,
    Input,
    Artefact,
    Finding,
    BuildError,
    DecodeError,
    ToolError,
> = LlmRoleAdapter<
    C,
    R,
    Build,
    Decode,
    RepairAttempts<Input, Artefact, Finding>,
    Input,
    BuildError,
    DecodeError,
    ToolError,
>;
type RepairAttempts<Input, Artefact, Finding> = Vec<Attempt<Input, Artefact, Finding>>;

struct LlmRoleAdapter<C, R, Build, Decode, Input, Output, BuildError, DecodeError, ToolError> {
    executor: Arc<Executor<C, R, ToolError>>,
    build_request: Build,
    decode_output: Decode,
    marker: AdapterMarker<Input, Output, BuildError, DecodeError>,
}

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
    adapter: LlmRoleAdapter<C, R, Build, Decode, Input, Output, BuildError, DecodeError, ToolError>,
}

/// A generic `naaf_core::Check` backed by an LLM executor.
pub struct LlmCheck<
    C,
    R,
    Build,
    Decode,
    Subject,
    Finding,
    BuildError,
    DecodeError,
    ToolError = Infallible,
> {
    adapter: LlmRoleAdapter<
        C,
        R,
        Build,
        Decode,
        Subject,
        Vec<Finding>,
        BuildError,
        DecodeError,
        ToolError,
    >,
}

/// A generic `naaf_core::Materialiser` backed by an LLM executor.
pub struct LlmMaterialiser<
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
    adapter: LlmRoleAdapter<C, R, Build, Decode, Input, Output, BuildError, DecodeError, ToolError>,
}

/// A generic `naaf_core::RepairPlanner` backed by an LLM executor.
pub struct LlmRepairPlanner<
    C,
    R,
    Build,
    Decode,
    Input,
    Artefact,
    Finding,
    BuildError,
    DecodeError,
    ToolError = Infallible,
> {
    adapter: RepairAdapter<
        C,
        R,
        Build,
        Decode,
        Input,
        Artefact,
        Finding,
        BuildError,
        DecodeError,
        ToolError,
    >,
}

impl<C, R, Build, Decode, Input, Output, BuildError, DecodeError, ToolError>
    LlmRoleAdapter<C, R, Build, Decode, Input, Output, BuildError, DecodeError, ToolError>
{
    fn from_shared_executor(
        executor: Arc<Executor<C, R, ToolError>>,
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

    fn executor(&self) -> &Executor<C, R, ToolError> {
        self.executor.as_ref()
    }
}

impl<C, R, Build, Decode, Input, Output, BuildError, DecodeError, ToolError>
    LlmRoleAdapter<C, R, Build, Decode, Input, Output, BuildError, DecodeError, ToolError>
where
    C: LlmClient<Runtime = R> + 'static,
    C::Error: 'static,
    BuildError: 'static,
    DecodeError: 'static,
    ToolError: 'static,
    Build: Fn(&R, Input) -> Result<CompletionRequest, BuildError> + 'static,
    Decode: Fn(ExecutionOutcome) -> Result<Output, DecodeError> + 'static,
{
    fn execute<'a>(
        &'a self,
        runtime: &'a R,
        input: Input,
    ) -> AdapterFuture<'a, C, Output, BuildError, ToolError, DecodeError> {
        Box::pin(async move {
            let request = (self.build_request)(runtime, input).map_err(AdaptorError::Build)?;
            let outcome = self
                .executor
                .execute(runtime, request)
                .await
                .map_err(AdaptorError::Execute)?;
            (self.decode_output)(outcome).map_err(AdaptorError::Decode)
        })
    }
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
        Self::with_executor(executor, build_request, decode_output)
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
        Self::from_shared_executor(Arc::new(executor), build_request, decode_output)
    }

    pub(crate) fn from_shared_executor(
        executor: Arc<Executor<C, R, ToolError>>,
        build_request: Build,
        decode_output: Decode,
    ) -> Self {
        Self {
            adapter: LlmRoleAdapter::from_shared_executor(executor, build_request, decode_output),
        }
    }

    /// Returns the underlying executor.
    pub fn executor(&self) -> &Executor<C, R, ToolError> {
        self.adapter.executor()
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
    type Error = AdaptorError<BuildError, C::Error, ToolError, DecodeError>;

    fn run<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        input: Self::Input,
    ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
        self.adapter.execute(runtime, input)
    }
}

impl<C, R, Build, Decode, Subject, Finding, BuildError, DecodeError>
    LlmCheck<C, R, Build, Decode, Subject, Finding, BuildError, DecodeError, Infallible>
{
    /// Creates an LLM check without tools.
    pub fn new(
        executor: Executor<C, R, Infallible>,
        build_request: Build,
        decode_findings: Decode,
    ) -> Self {
        Self::with_executor(executor, build_request, decode_findings)
    }
}

impl<C, R, Build, Decode, Subject, Finding, BuildError, DecodeError, ToolError>
    LlmCheck<C, R, Build, Decode, Subject, Finding, BuildError, DecodeError, ToolError>
{
    /// Creates an LLM check with a preconfigured executor.
    pub fn with_executor(
        executor: Executor<C, R, ToolError>,
        build_request: Build,
        decode_findings: Decode,
    ) -> Self {
        Self::from_shared_executor(Arc::new(executor), build_request, decode_findings)
    }

    pub(crate) fn from_shared_executor(
        executor: Arc<Executor<C, R, ToolError>>,
        build_request: Build,
        decode_findings: Decode,
    ) -> Self {
        Self {
            adapter: LlmRoleAdapter::from_shared_executor(executor, build_request, decode_findings),
        }
    }

    /// Returns the underlying executor.
    pub fn executor(&self) -> &Executor<C, R, ToolError> {
        self.adapter.executor()
    }
}

impl<C, R, Build, Decode, Subject, Finding, BuildError, DecodeError, ToolError> Check
    for LlmCheck<C, R, Build, Decode, Subject, Finding, BuildError, DecodeError, ToolError>
where
    C: LlmClient<Runtime = R> + 'static,
    C::Error: 'static,
    R: 'static,
    Subject: 'static,
    Finding: 'static,
    BuildError: 'static,
    DecodeError: 'static,
    ToolError: 'static,
    Build: Fn(&R, Subject) -> Result<CompletionRequest, BuildError> + 'static,
    Decode: Fn(ExecutionOutcome) -> Result<Vec<Finding>, DecodeError> + 'static,
{
    type Runtime = R;
    type Subject = Subject;
    type Finding = Finding;
    type Error = AdaptorError<BuildError, C::Error, ToolError, DecodeError>;

    fn check<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        subject: Self::Subject,
    ) -> LocalBoxFuture<'a, Result<Vec<Self::Finding>, Self::Error>> {
        self.adapter.execute(runtime, subject)
    }
}

impl<C, R, Build, Decode, Input, Output, BuildError, DecodeError>
    LlmMaterialiser<C, R, Build, Decode, Input, Output, BuildError, DecodeError, Infallible>
{
    /// Creates an LLM materialiser without tools.
    pub fn new(
        executor: Executor<C, R, Infallible>,
        build_request: Build,
        decode_output: Decode,
    ) -> Self {
        Self::with_executor(executor, build_request, decode_output)
    }
}

impl<C, R, Build, Decode, Input, Output, BuildError, DecodeError, ToolError>
    LlmMaterialiser<C, R, Build, Decode, Input, Output, BuildError, DecodeError, ToolError>
{
    /// Creates an LLM materialiser with a preconfigured executor.
    pub fn with_executor(
        executor: Executor<C, R, ToolError>,
        build_request: Build,
        decode_output: Decode,
    ) -> Self {
        Self::from_shared_executor(Arc::new(executor), build_request, decode_output)
    }

    pub(crate) fn from_shared_executor(
        executor: Arc<Executor<C, R, ToolError>>,
        build_request: Build,
        decode_output: Decode,
    ) -> Self {
        Self {
            adapter: LlmRoleAdapter::from_shared_executor(executor, build_request, decode_output),
        }
    }

    /// Returns the underlying executor.
    pub fn executor(&self) -> &Executor<C, R, ToolError> {
        self.adapter.executor()
    }
}

impl<C, R, Build, Decode, Input, Output, BuildError, DecodeError, ToolError> Materialiser
    for LlmMaterialiser<C, R, Build, Decode, Input, Output, BuildError, DecodeError, ToolError>
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
    type Error = AdaptorError<BuildError, C::Error, ToolError, DecodeError>;

    fn materialise<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        input: Self::Input,
    ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
        self.adapter.execute(runtime, input)
    }
}

impl<C, R, Build, Decode, Input, Artefact, Finding, BuildError, DecodeError>
    LlmRepairPlanner<
        C,
        R,
        Build,
        Decode,
        Input,
        Artefact,
        Finding,
        BuildError,
        DecodeError,
        Infallible,
    >
{
    /// Creates an LLM repair planner without tools.
    pub fn new(
        executor: Executor<C, R, Infallible>,
        build_request: Build,
        decode_input: Decode,
    ) -> Self {
        Self::with_executor(executor, build_request, decode_input)
    }
}

impl<C, R, Build, Decode, Input, Artefact, Finding, BuildError, DecodeError, ToolError>
    LlmRepairPlanner<
        C,
        R,
        Build,
        Decode,
        Input,
        Artefact,
        Finding,
        BuildError,
        DecodeError,
        ToolError,
    >
{
    /// Creates an LLM repair planner with a preconfigured executor.
    pub fn with_executor(
        executor: Executor<C, R, ToolError>,
        build_request: Build,
        decode_input: Decode,
    ) -> Self {
        Self::from_shared_executor(Arc::new(executor), build_request, decode_input)
    }

    pub(crate) fn from_shared_executor(
        executor: Arc<Executor<C, R, ToolError>>,
        build_request: Build,
        decode_input: Decode,
    ) -> Self {
        Self {
            adapter: LlmRoleAdapter::from_shared_executor(executor, build_request, decode_input),
        }
    }

    /// Returns the underlying executor.
    pub fn executor(&self) -> &Executor<C, R, ToolError> {
        self.adapter.executor()
    }
}

impl<C, R, Build, Decode, Input, Artefact, Finding, BuildError, DecodeError, ToolError>
    RepairPlanner
    for LlmRepairPlanner<
        C,
        R,
        Build,
        Decode,
        Input,
        Artefact,
        Finding,
        BuildError,
        DecodeError,
        ToolError,
    >
where
    C: LlmClient<Runtime = R> + 'static,
    C::Error: 'static,
    R: 'static,
    Input: 'static,
    Artefact: 'static,
    Finding: 'static,
    BuildError: 'static,
    DecodeError: 'static,
    ToolError: 'static,
    Build: Fn(&R, Vec<Attempt<Input, Artefact, Finding>>) -> Result<CompletionRequest, BuildError>
        + 'static,
    Decode: Fn(ExecutionOutcome) -> Result<Input, DecodeError> + 'static,
{
    type Runtime = R;
    type Input = Input;
    type Artefact = Artefact;
    type Finding = Finding;
    type Error = AdaptorError<BuildError, C::Error, ToolError, DecodeError>;

    fn repair<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        attempts: Vec<Attempt<Self::Input, Self::Artefact, Self::Finding>>,
    ) -> LocalBoxFuture<'a, Result<Self::Input, Self::Error>> {
        self.adapter.execute(runtime, attempts)
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
    use naaf_core::{Attempt, Check, Materialiser, RepairPlanner, Task};
    use serde_json::{Value, json};

    use super::{LlmCheck, LlmMaterialiser, LlmRepairPlanner, LlmTask};
    use crate::{
        AssistantMessage, CompletionRequest, CompletionResponse, ExecutionOutcome, Executor,
        LlmAgent, LlmClient, Message, Tool, ToolCall, ToolRegistry, ToolSpec,
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

    #[tokio::test]
    async fn llm_agent_projects_task_and_check_from_shared_executor() {
        let client = StubClient::new(vec![
            CompletionResponse::new(AssistantMessage::from_text("plan ready")),
            CompletionResponse::new(AssistantMessage::from_text("[]")),
        ]);
        let agent = LlmAgent::new(client);

        let task = agent.task(
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
        let check = agent.check(
            |_runtime: &TestRuntime, subject: String| {
                Ok::<_, Infallible>(CompletionRequest::new(
                    "test-model",
                    vec![Message::user(subject)],
                ))
            },
            |outcome: ExecutionOutcome| {
                serde_json::from_str::<Vec<String>>(
                    outcome.final_message().content.as_deref().unwrap_or("[]"),
                )
            },
        );

        let output = task
            .run(&TestRuntime, "draft a plan".to_string())
            .await
            .expect("task should succeed");
        let findings = check
            .check(&TestRuntime, output.clone())
            .await
            .expect("check should succeed");

        assert_eq!(output, "plan ready");
        assert!(findings.is_empty());
        assert_eq!(agent.executor().client().requests().len(), 2);
    }

    #[tokio::test]
    async fn llm_materialiser_decodes_structured_output() {
        let client = StubClient::new(vec![CompletionResponse::new(AssistantMessage::from_text(
            r#"{"revision":2}"#,
        ))]);
        let materialiser = LlmMaterialiser::new(
            Executor::new(client),
            |_runtime: &TestRuntime, input: usize| {
                Ok::<_, Infallible>(CompletionRequest::new(
                    "test-model",
                    vec![Message::user(format!("materialise revision {input}"))],
                ))
            },
            |outcome: ExecutionOutcome| {
                serde_json::from_str::<serde_json::Value>(
                    outcome.final_message().content.as_deref().unwrap_or("null"),
                )
            },
        );

        let output = materialiser
            .materialise(&TestRuntime, 2)
            .await
            .expect("materialiser should succeed");

        assert_eq!(output["revision"], 2);
    }

    #[tokio::test]
    async fn llm_repair_planner_decodes_next_input() {
        let client = StubClient::new(vec![CompletionResponse::new(AssistantMessage::from_text(
            r#"{"revision":3}"#,
        ))]);
        let planner = LlmRepairPlanner::new(
            Executor::new(client),
            |_runtime: &TestRuntime, attempts: Vec<Attempt<usize, usize, String>>| {
                Ok::<_, Infallible>(CompletionRequest::new(
                    "test-model",
                    vec![Message::user(format!(
                        "repair attempt count {}",
                        attempts.len()
                    ))],
                ))
            },
            |outcome: ExecutionOutcome| {
                outcome
                    .final_message()
                    .content
                    .as_deref()
                    .unwrap_or("{}")
                    .parse::<serde_json::Value>()
                    .map(|value| value["revision"].as_u64().unwrap_or_default() as usize)
                    .map_err(|error| TestError(Box::leak(error.to_string().into_boxed_str())))
            },
        );

        let next = planner
            .repair(
                &TestRuntime,
                vec![Attempt {
                    input: 1,
                    artefact: 2,
                    findings: vec!["tests failed".to_string()],
                }],
            )
            .await
            .expect("repair planner should succeed");

        assert_eq!(next, 3);
    }

    #[tokio::test]
    async fn llm_check_supports_direct_construction() {
        let client = StubClient::new(vec![CompletionResponse::new(AssistantMessage::from_text(
            r#"["missing tests"]"#,
        ))]);
        let check = LlmCheck::new(
            Executor::new(client),
            |_runtime: &TestRuntime, subject: String| {
                Ok::<_, Infallible>(CompletionRequest::new(
                    "test-model",
                    vec![Message::user(subject)],
                ))
            },
            |outcome: ExecutionOutcome| {
                serde_json::from_str::<Vec<String>>(
                    outcome.final_message().content.as_deref().unwrap_or("[]"),
                )
            },
        );

        let findings = check
            .check(&TestRuntime, "review this patch".to_string())
            .await
            .expect("check should succeed");

        assert_eq!(findings, vec!["missing tests".to_string()]);
    }
}
