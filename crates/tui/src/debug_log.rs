use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crossterm::event::{KeyCode, KeyEvent};

use crate::event::TuiEvent;
use crate::ui::{AppState, KeyAction, TuiPhase};

pub struct DebugLog {
    writer: BufWriter<File>,
    sequence: u64,
}

impl DebugLog {
    pub fn open(path: PathBuf) -> io::Result<Self> {
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self {
            writer: BufWriter::new(file),
            sequence: 0,
        })
    }

    pub fn record_launch(
        &mut self,
        title: &str,
        tick_rate_ms: u64,
        max_log_lines: usize,
        input_label: Option<&str>,
    ) -> io::Result<()> {
        self.record(
            "launch",
            format!(
                "title={title:?} tick_rate_ms={tick_rate_ms} max_log_lines={max_log_lines} input_label={input_label:?}"
            ),
        )
    }

    pub fn record_event(&mut self, event: &TuiEvent) -> io::Result<()> {
        self.record("event", describe_event(event))
    }

    pub fn record_state(&mut self, label: &str, state: &AppState) -> io::Result<()> {
        self.record(label, describe_state(state))
    }

    pub fn record_key(&mut self, key: &KeyEvent) -> io::Result<()> {
        self.record(
            "key",
            format!(
                "code={} modifiers={:?}",
                describe_key_code(key.code),
                key.modifiers
            ),
        )
    }

    pub fn record_key_action(&mut self, action: &KeyAction) -> io::Result<()> {
        match action {
            KeyAction::Continue => Ok(()),
            KeyAction::Quit => self.record("action", "quit_requested".to_string()),
            KeyAction::InstructionSubmitted(instruction) => self.record(
                "action",
                format!("instruction_submitted value={instruction:?}"),
            ),
            KeyAction::PromptSubmitted { question, reply } => self.record(
                "action",
                format!("prompt_submitted question={question:?} reply={reply:?}"),
            ),
        }
    }

    fn record(&mut self, label: &str, message: String) -> io::Result<()> {
        self.sequence += 1;
        writeln!(
            self.writer,
            "ts={} seq={} {} {}",
            unix_timestamp_millis(),
            self.sequence,
            label,
            message
        )?;
        self.writer.flush()
    }
}

fn unix_timestamp_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn describe_event(event: &TuiEvent) -> String {
    match event {
        TuiEvent::StepStarted {
            task_name,
            task_label,
        } => format!("step_started task_name={task_name:?} task_label={task_label:?}"),
        TuiEvent::StepAttemptStarted {
            task_name,
            task_label,
            attempt,
        } => format!(
            "step_attempt_started task_name={task_name:?} task_label={task_label:?} attempt={attempt}"
        ),
        TuiEvent::StepAttemptValidated {
            task_name,
            task_label,
            attempt,
            accepted,
            finding_count,
        } => format!(
            "step_attempt_validated task_name={task_name:?} task_label={task_label:?} attempt={attempt} accepted={accepted} finding_count={finding_count}"
        ),
        TuiEvent::StepRepairStarted {
            task_name,
            task_label,
            attempt,
        } => format!(
            "step_repair_started task_name={task_name:?} task_label={task_label:?} attempt={attempt}"
        ),
        TuiEvent::StepCompleted {
            task_name,
            task_label,
            attempts,
        } => format!(
            "step_completed task_name={task_name:?} task_label={task_label:?} attempts={attempts}"
        ),
        TuiEvent::StepRejected {
            task_name,
            task_label,
            attempts,
            reason,
        } => format!(
            "step_rejected task_name={task_name:?} task_label={task_label:?} attempts={attempts} reason={reason:?}"
        ),
        TuiEvent::StepFailed {
            task_name,
            task_label,
            stage,
        } => {
            format!("step_failed task_name={task_name:?} task_label={task_label:?} stage={stage:?}")
        }
        TuiEvent::ComponentStarted { component, name } => {
            format!("component_started component={component:?} name={name:?}")
        }
        TuiEvent::ComponentCompleted { component, name } => {
            format!("component_completed component={component:?} name={name:?}")
        }
        TuiEvent::ComponentFailed { component, name } => {
            format!("component_failed component={component:?} name={name:?}")
        }
        TuiEvent::Log {
            level,
            target,
            message,
        } => format!("log level={level} target={target:?} message={message:?}"),
        TuiEvent::HumanPrompt {
            question, choices, ..
        } => format!("human_prompt question={question:?} choices={choices:?}"),
        TuiEvent::Quit => "quit".to_string(),
    }
}

fn describe_state(state: &AppState) -> String {
    let phase = match &state.phase {
        TuiPhase::Input { buffer, cursor } => {
            format!(
                "input label={:?} buffer={buffer:?} cursor={cursor}",
                state.input_prompt_label
            )
        }
        TuiPhase::Running => "running".to_string(),
    };

    let active_prompt = match &state.active_prompt {
        Some(prompt) => format!(
            "question={:?} choices={:?} input={:?}",
            prompt.question, prompt.choices, prompt.input
        ),
        None => "none".to_string(),
    };

    let steps = state
        .steps
        .iter()
        .map(|step| format!("{:?}:{:?}", step.task_label, step.status))
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        "phase={phase} steps={} selected_step={} logs={} active_prompt={} step_statuses=[{}]",
        state.steps.len(),
        state.selected_step,
        state.logs.len(),
        active_prompt,
        steps
    )
}

fn describe_key_code(code: KeyCode) -> String {
    match code {
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Left => "Left".to_string(),
        KeyCode::Right => "Right".to_string(),
        KeyCode::Up => "Up".to_string(),
        KeyCode::Down => "Down".to_string(),
        KeyCode::Home => "Home".to_string(),
        KeyCode::End => "End".to_string(),
        KeyCode::PageUp => "PageUp".to_string(),
        KeyCode::PageDown => "PageDown".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::BackTab => "BackTab".to_string(),
        KeyCode::Delete => "Delete".to_string(),
        KeyCode::Insert => "Insert".to_string(),
        KeyCode::F(n) => format!("F({n})"),
        KeyCode::Char(c) => format!("Char({c:?})"),
        KeyCode::Null => "Null".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        other => format!("{other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use tokio::sync::oneshot;

    use super::DebugLog;
    use crate::event::TuiEvent;
    use crate::ui::AppState;

    fn temp_log_path(name: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("naaf-tui-{name}-{unique}.log"))
    }

    #[test]
    fn debug_log_records_prompt_events_and_state() {
        let path = temp_log_path("prompt-events");
        let (reply, _rx) = oneshot::channel();
        let mut log = DebugLog::open(path.clone()).expect("debug log should open");
        let mut state = AppState::new("naaf".to_string(), 100);

        let event = TuiEvent::HumanPrompt {
            question: "Need clarification?".to_string(),
            choices: vec!["yes".to_string(), "no".to_string()],
            reply,
        };

        log.record_event(&event).expect("event should be written");
        state.handle_event(event);
        log.record_state("after_event", &state)
            .expect("state should be written");

        let contents = fs::read_to_string(&path).expect("debug log should be readable");
        fs::remove_file(&path).ok();

        assert!(contents.contains("event human_prompt question=\"Need clarification?\""));
        assert!(contents.contains("after_event phase=running"));
        assert!(contents.contains("active_prompt=question=\"Need clarification?\""));
    }
}
