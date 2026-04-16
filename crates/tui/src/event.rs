use std::fmt;

#[derive(Clone, Debug)]
pub enum TuiEvent {
    StepStarted {
        task_name: String,
    },
    StepAttemptStarted {
        task_name: String,
        attempt: usize,
    },
    StepAttemptValidated {
        task_name: String,
        attempt: usize,
        accepted: bool,
        finding_count: usize,
    },
    StepRepairStarted {
        task_name: String,
        attempt: usize,
    },
    StepCompleted {
        task_name: String,
        attempts: usize,
    },
    StepRejected {
        task_name: String,
        attempts: usize,
        reason: String,
    },
    StepFailed {
        task_name: String,
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
        level: tracing::Level,
        target: String,
        message: String,
    },
    HumanPrompt {
        question: String,
        choices: Vec<String>,
    },
    HumanAnswer {
        answer: String,
    },
    Quit,
}

impl fmt::Display for TuiEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StepStarted { task_name } => write!(f, "step started: {task_name}"),
            Self::StepAttemptStarted { task_name, attempt } => {
                write!(f, "attempt {attempt} started: {task_name}")
            }
            Self::StepAttemptValidated {
                task_name,
                attempt,
                accepted,
                finding_count,
            } => write!(
                f,
                "attempt {attempt} validated: {task_name} (accepted={accepted}, findings={finding_count})"
            ),
            Self::StepRepairStarted { task_name, attempt } => {
                write!(f, "repair started: {task_name} (attempt {attempt})")
            }
            Self::StepCompleted {
                task_name,
                attempts,
            } => {
                write!(f, "step completed: {task_name} ({attempts} attempts)")
            }
            Self::StepRejected {
                task_name,
                attempts,
                reason,
            } => write!(
                f,
                "step rejected: {task_name} ({attempts} attempts, {reason})"
            ),
            Self::StepFailed { task_name, stage } => {
                write!(f, "step failed: {task_name} ({stage})")
            }
            Self::ComponentStarted { component, name } => {
                write!(f, "{component} started: {name}")
            }
            Self::ComponentCompleted { component, name } => {
                write!(f, "{component} completed: {name}")
            }
            Self::ComponentFailed { component, name } => {
                write!(f, "{component} failed: {name}")
            }
            Self::Log {
                level,
                target,
                message,
            } => write!(f, "[{level}] {target}: {message}"),
            Self::HumanPrompt { question, .. } => write!(f, "? {question}"),
            Self::HumanAnswer { answer } => write!(f, "answer: {answer}"),
            Self::Quit => write!(f, "quit"),
        }
    }
}
