use std::{convert::Infallible, sync::Arc};

use naaf_core::{Task, TaskExt};
use serde::de::DeserializeOwned;
use serde_json::json;

use crate::{
    ExecutionOutcome,
    executor::Executor,
    message::{CompletionRequest, Message},
    message_source::MessageSource,
    task::{LlmCheck, LlmMaterialiser, LlmRepairPlanner, LlmTask},
};

/// Shared LLM backend that can be projected into `naaf_core` roles.
#[derive(Clone)]
pub struct LlmAgent<C, R, E = Infallible> {
    executor: Arc<Executor<C, R, E>>,
}

impl<C, R: 'static> LlmAgent<C, R, Infallible> {
    /// Creates an LLM agent without tools.
    pub fn new(client: C) -> Self {
        Self::with_executor(Executor::new(client))
    }
}

impl<C, R: 'static, E> LlmAgent<C, R, E> {
    /// Creates an LLM agent with a preconfigured executor.
    pub fn with_executor(executor: Executor<C, R, E>) -> Self {
        Self {
            executor: Arc::new(executor),
        }
    }

    /// Returns the underlying executor.
    pub fn executor(&self) -> &Executor<C, R, E> {
        self.executor.as_ref()
    }

    /// Returns the message source configured on the underlying executor, if any.
    pub fn message_source(&self) -> Option<&Arc<dyn MessageSource>> {
        self.executor.message_source()
    }

    /// Returns a new agent with the given message source attached to its
    /// executor. The source is used by the executor to drain queued user
    /// messages between LLM turns.
    pub fn with_message_source(self, source: Arc<dyn MessageSource>) -> Self
    where
        C: Clone,
        E: Clone,
    {
        let mut executor = Arc::try_unwrap(self.executor).unwrap_or_else(|arc| {
            let matched = Arc::into_inner(arc);
            debug_assert!(
                matched.is_some(),
                "LlmAgent with_message_source called while executor is shared"
            );
            matched.unwrap()
        });
        executor = executor.with_message_source(source);
        Self {
            executor: Arc::new(executor),
        }
    }

    /// Projects this agent into a `naaf_core::Task`.
    pub fn task<Build, Decode, Input, Output, BuildError, DecodeError>(
        &self,
        build_request: Build,
        decode_output: Decode,
    ) -> LlmTask<C, R, Build, Decode, Input, Output, BuildError, DecodeError, E> {
        LlmTask::from_shared_executor(self.executor.clone(), build_request, decode_output)
    }

    /// Projects this agent into a `naaf_core::Check`.
    pub fn check<Build, Decode, Input, Output, Finding, BuildError, DecodeError>(
        &self,
        build_request: Build,
        decode_findings: Decode,
    ) -> LlmCheck<C, R, Build, Decode, Input, Output, Finding, BuildError, DecodeError, E> {
        LlmCheck::from_shared_executor(self.executor.clone(), build_request, decode_findings)
    }

    /// Projects this agent into a `naaf_core::Materialiser`.
    pub fn materialiser<Build, Decode, Input, Output, BuildError, DecodeError>(
        &self,
        build_request: Build,
        decode_output: Decode,
    ) -> LlmMaterialiser<C, R, Build, Decode, Input, Output, BuildError, DecodeError, E> {
        LlmMaterialiser::from_shared_executor(self.executor.clone(), build_request, decode_output)
    }

    /// Projects this agent into a `naaf_core::RepairPlanner`.
    pub fn repair_planner<Build, Decode, Input, Output, Finding, BuildError, DecodeError>(
        &self,
        build_request: Build,
        decode_input: Decode,
    ) -> LlmRepairPlanner<C, R, Build, Decode, Input, Output, Finding, BuildError, DecodeError, E>
    {
        LlmRepairPlanner::from_shared_executor(self.executor.clone(), build_request, decode_input)
    }

    /// Creates a JSON-decoding task from a system prompt, user-prompt builder, and output type.
    ///
    /// This is a convenience wrapper for the common pattern of:
    /// - sending a static system prompt
    /// - building a user prompt from the input
    /// - decoding the assistant response as JSON into `Output`
    pub fn json_task<Input, Output, BuildUser, BuildError>(
        &self,
        model: String,
        system_prompt: String,
        build_user_prompt: BuildUser,
        decode: fn(ExecutionOutcome) -> Result<Output, serde_json::Error>,
        label: String,
    ) -> impl Task<
        Runtime = R,
        Input = Input,
        Output = Output,
        Error = crate::error::AdaptorError<BuildError, C::Error, E, serde_json::Error>,
    > + use<C, R, E, Input, Output, BuildError, BuildUser>
    where
        C: crate::client::LlmClient<Runtime = R> + 'static,
        C::Error: std::fmt::Debug + 'static,
        E: std::fmt::Debug + 'static,
        BuildError: std::fmt::Debug + 'static,
        BuildUser: Fn(Input) -> Result<String, BuildError> + 'static,
        Output: DeserializeOwned + std::fmt::Debug + 'static,
        Input: std::fmt::Debug + 'static,
    {
        self.task(
            move |_runtime: &R, input: Input| {
                let user_content = build_user_prompt(input)?;
                Ok::<_, BuildError>(
                    CompletionRequest::new(
                        model.clone(),
                        vec![
                            Message::system(system_prompt.clone()),
                            Message::user(user_content),
                        ],
                    )
                    .with_metadata(json!({
                        "response_format": json_task_response_format()
                    })),
                )
            },
            decode,
        )
        .observed_as(label)
    }
}

fn json_task_response_format() -> serde_json::Value {
    json!({
        "type": "json_schema",
        "json_schema": {
            "name": "json_task_output",
            "strict": false,
            "schema": {
                "type": "object",
                "properties": {},
                "additionalProperties": true
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use std::{
        convert::Infallible,
        sync::{Arc, Mutex},
    };

    use futures::future::LocalBoxFuture;
    use naaf_core::Task;
    use serde_json::{Value, json};

    use crate::{
        client::LlmClient,
        message::{AssistantMessage, CompletionResponse},
    };

    use super::*;

    #[derive(Clone, Default)]
    struct CapturingClient {
        requests: Arc<Mutex<Vec<CompletionRequest>>>,
    }

    impl CapturingClient {
        fn requests(&self) -> Vec<CompletionRequest> {
            self.requests.lock().expect("request lock poisoned").clone()
        }
    }

    impl LlmClient for CapturingClient {
        type Error = Infallible;
        type Runtime = ();

        fn complete<'a>(
            &'a self,
            _runtime: &'a Self::Runtime,
            request: CompletionRequest,
        ) -> LocalBoxFuture<'a, Result<CompletionResponse, Self::Error>> {
            self.requests
                .lock()
                .expect("request lock poisoned")
                .push(request);

            Box::pin(async {
                Ok(CompletionResponse::new(AssistantMessage::from_text(
                    "{\"ok\":true}",
                )))
            })
        }
    }

    #[tokio::test]
    async fn json_task_requests_json_schema_response_format() {
        let client = CapturingClient::default();
        let agent = LlmAgent::new(client.clone());
        let task = agent.json_task(
            "model".to_string(),
            "system".to_string(),
            |input: &str| Ok::<_, Infallible>(input.to_string()),
            |outcome| {
                serde_json::from_str::<Value>(
                    outcome
                        .final_message()
                        .content
                        .as_deref()
                        .expect("assistant content should exist"),
                )
            },
            "json-test".to_string(),
        );

        let output = task
            .run(&(), "user")
            .await
            .expect("JSON task should complete");

        assert_eq!(output, json!({ "ok": true }));
        assert_eq!(
            client.requests()[0].metadata["response_format"],
            json!({
                "type": "json_schema",
                "json_schema": {
                    "name": "json_task_output",
                    "strict": false,
                    "schema": {
                        "type": "object",
                        "properties": {},
                        "additionalProperties": true
                    }
                }
            })
        );
    }
}
