use serde::{Deserialize, Serialize};

use crate::frontend_event::FrontendEvent;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsEvent {
    StepStarted {
        task_name: String,
        task_label: String,
    },
    StepAttemptStarted {
        task_name: String,
        task_label: String,
        attempt: usize,
    },
    StepAttemptValidated {
        task_name: String,
        task_label: String,
        attempt: usize,
        accepted: bool,
        finding_count: usize,
    },
    StepRepairStarted {
        task_name: String,
        task_label: String,
        attempt: usize,
    },
    StepCompleted {
        task_name: String,
        task_label: String,
        attempts: usize,
    },
    StepRejected {
        task_name: String,
        task_label: String,
        attempts: usize,
        reason: String,
    },
    StepFailed {
        task_name: String,
        task_label: String,
        stage: String,
    },
    ComponentStarted {
        component: String,
        name: String,
    },
    ComponentCompleted {
        component: String,
        name: String,
    },
    ComponentFailed {
        component: String,
        name: String,
    },
    Log {
        level: String,
        target: String,
        message: String,
    },
    HumanPrompt {
        prompt_id: String,
        question: String,
        choices: Vec<String>,
    },
    InputRequested {
        label: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsClientMessage {
    HumanPromptReply { prompt_id: String, reply: String },
    InputReply { reply: String },
}

impl WsEvent {
    pub fn from_frontend_event(event: FrontendEvent) -> Option<Self> {
        match event {
            FrontendEvent::StepStarted {
                task_name,
                task_label,
            } => Some(Self::StepStarted {
                task_name,
                task_label,
            }),
            FrontendEvent::StepAttemptStarted {
                task_name,
                task_label,
                attempt,
            } => Some(Self::StepAttemptStarted {
                task_name,
                task_label,
                attempt,
            }),
            FrontendEvent::StepAttemptValidated {
                task_name,
                task_label,
                attempt,
                accepted,
                finding_count,
            } => Some(Self::StepAttemptValidated {
                task_name,
                task_label,
                attempt,
                accepted,
                finding_count,
            }),
            FrontendEvent::StepRepairStarted {
                task_name,
                task_label,
                attempt,
            } => Some(Self::StepRepairStarted {
                task_name,
                task_label,
                attempt,
            }),
            FrontendEvent::StepCompleted {
                task_name,
                task_label,
                attempts,
            } => Some(Self::StepCompleted {
                task_name,
                task_label,
                attempts,
            }),
            FrontendEvent::StepRejected {
                task_name,
                task_label,
                attempts,
                reason,
            } => Some(Self::StepRejected {
                task_name,
                task_label,
                attempts,
                reason,
            }),
            FrontendEvent::StepFailed {
                task_name,
                task_label,
                stage,
            } => Some(Self::StepFailed {
                task_name,
                task_label,
                stage,
            }),
            FrontendEvent::ComponentStarted { component, name } => {
                Some(Self::ComponentStarted { component, name })
            }
            FrontendEvent::ComponentCompleted { component, name } => {
                Some(Self::ComponentCompleted { component, name })
            }
            FrontendEvent::ComponentFailed { component, name } => {
                Some(Self::ComponentFailed { component, name })
            }
            FrontendEvent::Log {
                level,
                target,
                message,
            } => Some(Self::Log {
                level: Self::level_name(&level).to_string(),
                target,
                message,
            }),
            FrontendEvent::HumanPrompt { .. } | FrontendEvent::Quit => None,
        }
    }

    pub fn level_name(level: &tracing::Level) -> &'static str {
        match *level {
            tracing::Level::ERROR => "ERROR",
            tracing::Level::WARN => "WARN",
            tracing::Level::INFO => "INFO",
            tracing::Level::DEBUG => "DEBUG",
            tracing::Level::TRACE => "TRACE",
        }
    }
}
