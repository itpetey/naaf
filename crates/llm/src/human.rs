use std::{convert::Infallible, io::Write, marker::PhantomData, sync::Arc};

use futures::future::LocalBoxFuture;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::Mutex;

use crate::message::ToolSpec;
use crate::tool::Tool;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HumanQuestion {
    pub question: String,
    #[serde(default)]
    pub choices: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HumanAnswer {
    pub content: String,
}

pub trait HumanIO {
    type Error;
    fn ask<'a>(
        &'a self,
        question: HumanQuestion,
    ) -> LocalBoxFuture<'a, Result<HumanAnswer, Self::Error>>;
}

#[derive(Debug, Error)]
pub enum StdinError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid choice: {0}")]
    InvalidChoice(String),
}

pub struct StdinHumanIO {
    reader: Arc<Mutex<BufReader<tokio::io::Stdin>>>,
}

impl StdinHumanIO {
    pub fn new() -> Self {
        Self {
            reader: Arc::new(Mutex::new(BufReader::new(tokio::io::stdin()))),
        }
    }
}

impl Default for StdinHumanIO {
    fn default() -> Self {
        Self::new()
    }
}

impl HumanIO for StdinHumanIO {
    type Error = StdinError;

    fn ask<'a>(
        &'a self,
        question: HumanQuestion,
    ) -> LocalBoxFuture<'a, Result<HumanAnswer, Self::Error>> {
        Box::pin(async move {
            let mut stdout = std::io::stdout().lock();
            writeln!(stdout, "{}", question.question)?;
            if let Some(ref choices) = question.choices {
                for (i, choice) in choices.iter().enumerate() {
                    writeln!(stdout, "  {}: {}", i + 1, choice)?;
                }
            }
            write!(stdout, "> ")?;
            stdout.flush()?;

            let mut line = String::new();
            {
                let mut reader = self.reader.lock().await;
                reader.read_line(&mut line).await?;
            }

            let trimmed = line.trim();
            let content = if let Some(ref choices) = question.choices {
                if let Ok(index) = trimmed.parse::<usize>() {
                    choices
                        .get(index - 1)
                        .ok_or_else(|| StdinError::InvalidChoice(trimmed.to_string()))?
                        .clone()
                } else {
                    trimmed.to_string()
                }
            } else {
                trimmed.to_string()
            };

            Ok(HumanAnswer { content })
        })
    }
}

pub struct PendingQuestion {
    pub question: HumanQuestion,
    pub reply: tokio::sync::oneshot::Sender<HumanAnswer>,
}

pub struct ChannelHumanIO {
    pending: tokio::sync::mpsc::Sender<PendingQuestion>,
}

impl ChannelHumanIO {
    pub fn new(buffer: usize) -> (Self, tokio::sync::mpsc::Receiver<PendingQuestion>) {
        let (tx, rx) = tokio::sync::mpsc::channel(buffer);
        (Self { pending: tx }, rx)
    }
}

impl HumanIO for ChannelHumanIO {
    type Error = Infallible;

    fn ask<'a>(
        &'a self,
        question: HumanQuestion,
    ) -> LocalBoxFuture<'a, Result<HumanAnswer, Self::Error>> {
        Box::pin(async move {
            let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
            let pending = PendingQuestion {
                question,
                reply: reply_tx,
            };
            self.pending
                .send(pending)
                .await
                .expect("receiver held open by caller");
            Ok(reply_rx.await.expect("reply channel not dropped"))
        })
    }
}

pub struct QuestionTool<R> {
    _marker: PhantomData<R>,
}

impl<R> QuestionTool<R> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<R> Default for QuestionTool<R> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R> Tool for QuestionTool<R>
where
    R: HumanIO,
    <R as HumanIO>::Error: 'static,
{
    type Runtime = R;
    type Error = <R as HumanIO>::Error;

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "ask_user".to_string(),
            description: "Ask the user a question and wait for their response. Use this when you need clarification, a decision, or information only the user can provide.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "question": {
                        "type": "string",
                        "description": "The question to present to the user",
                    },
                    "choices": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Optional list of choices for the user to select from",
                    },
                },
                "required": ["question"],
            }),
        }
    }

    fn call<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        arguments: Value,
    ) -> LocalBoxFuture<'a, Result<Value, Self::Error>> {
        Box::pin(async move {
            let question: HumanQuestion = match serde_json::from_value(arguments) {
                Ok(q) => q,
                Err(e) => {
                    return Ok(serde_json::json!({
                        "error": format!("invalid arguments: {e}"),
                    }));
                }
            };

            let answer = runtime.ask(question).await?;
            Ok(serde_json::json!({
                "answer": answer.content,
            }))
        })
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use serde_json::json;

    use super::*;

    #[derive(Debug)]
    struct StubRuntime {
        canned_answer: String,
    }

    impl HumanIO for StubRuntime {
        type Error = Infallible;

        fn ask<'a>(
            &'a self,
            _question: HumanQuestion,
        ) -> LocalBoxFuture<'a, Result<HumanAnswer, Self::Error>> {
            let answer = self.canned_answer.clone();
            Box::pin(async move { Ok(HumanAnswer { content: answer }) })
        }
    }

    #[test]
    fn question_tool_spec_has_correct_name_and_schema() {
        let tool = QuestionTool::<StubRuntime>::new();
        let spec = tool.spec();

        assert_eq!(spec.name, "ask_user");
        let schema = &spec.input_schema;
        let required = schema["required"].as_array().unwrap();
        assert!(required.iter().any(|r| r == "question"));
        assert!(schema["properties"]["question"]["type"].is_string());
        assert!(schema["properties"]["choices"].is_object());
    }

    #[tokio::test]
    async fn question_tool_returns_answer_from_runtime() {
        let runtime = StubRuntime {
            canned_answer: "yes".to_string(),
        };
        let tool = QuestionTool::<StubRuntime>::new();

        let result = tool
            .call(&runtime, json!({ "question": "Should I proceed?" }))
            .await
            .unwrap();

        assert_eq!(result["answer"], "yes");
    }

    #[tokio::test]
    async fn question_tool_returns_error_on_invalid_arguments() {
        let runtime = StubRuntime {
            canned_answer: "unused".to_string(),
        };
        let tool = QuestionTool::<StubRuntime>::new();

        let result = tool.call(&runtime, json!("not an object")).await.unwrap();

        assert!(result["error"].is_string());
    }

    #[tokio::test]
    async fn question_tool_passes_choices_through() {
        let runtime = StubRuntime {
            canned_answer: "blue".to_string(),
        };
        let tool = QuestionTool::<StubRuntime>::new();

        let result = tool
            .call(
                &runtime,
                json!({
                    "question": "Favourite colour?",
                    "choices": ["red", "green", "blue"]
                }),
            )
            .await
            .unwrap();

        assert_eq!(result["answer"], "blue");
    }

    #[tokio::test]
    async fn channel_human_io_round_trip() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let (io, mut rx) = ChannelHumanIO::new(4);

                let ask_handle = tokio::task::spawn_local(async move {
                    io.ask(HumanQuestion {
                        question: "Pick a number".to_string(),
                        choices: None,
                    })
                    .await
                    .unwrap()
                });

                let pending = rx.recv().await.expect("pending question");
                assert_eq!(pending.question.question, "Pick a number");
                pending
                    .reply
                    .send(HumanAnswer {
                        content: "42".to_string(),
                    })
                    .expect("reply sent");

                let answer = ask_handle.await.unwrap();
                assert_eq!(answer.content, "42");
            })
            .await;
    }

    #[tokio::test]
    async fn channel_human_io_blocks_until_answered() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let (io, mut rx) = ChannelHumanIO::new(4);

                let ask_handle = tokio::task::spawn_local(async move {
                    io.ask(HumanQuestion {
                        question: "Waiting...".to_string(),
                        choices: None,
                    })
                    .await
                    .unwrap()
                });

                tokio::task::yield_now().await;
                assert!(!ask_handle.is_finished());

                let pending = rx.recv().await.expect("pending question");
                pending
                    .reply
                    .send(HumanAnswer {
                        content: "finally".to_string(),
                    })
                    .expect("reply sent");

                let answer = ask_handle.await.unwrap();
                assert_eq!(answer.content, "finally");
            })
            .await;
    }
}
