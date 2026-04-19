pub mod detail;
pub mod human;
pub mod input;
pub mod logs;
pub mod steps;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::event::TuiEvent;

const CTRL_C_QUIT_MESSAGE: &str = "press ctrl+c again to quit";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TuiPhase {
    Input { buffer: String, cursor: usize },
    Running,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyAction {
    Continue,
    QuitArmed,
    Quit,
    InstructionSubmitted(String),
    PromptSubmitted { question: String, reply: String },
}

pub struct AppState {
    pub title: String,
    pub phase: TuiPhase,
    pub steps: Vec<StepState>,
    pub selected_step: usize,
    pub logs: Vec<LogEntry>,
    pub max_log_lines: usize,
    pub log_scroll_offset: usize,
    pub active_prompt: Option<PromptState>,
    pub input_prompt_label: String,
    quit_armed: bool,
    instruction_tx: Option<tokio::sync::oneshot::Sender<String>>,
}

#[derive(Clone, Debug)]
pub struct StepState {
    pub task_name: String,
    pub task_label: String,
    pub status: StepStatus,
    pub current_attempt: usize,
    pub max_attempts: usize,
    pub attempt_findings: Vec<usize>,
    pub detail_lines: Vec<String>,
}

#[derive(Clone, Debug)]
pub enum StepStatus {
    Running,
    Completed,
    Rejected { reason: String },
    Failed { stage: String },
}

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub level: tracing::Level,
    pub message: String,
}

#[derive(Debug)]
pub struct PromptState {
    pub question: String,
    pub choices: Vec<String>,
    pub input: String,
    reply: tokio::sync::oneshot::Sender<String>,
}

impl AppState {
    pub fn new(title: String, max_log_lines: usize) -> Self {
        Self {
            title,
            phase: TuiPhase::Running,
            steps: Vec::new(),
            selected_step: 0,
            logs: Vec::new(),
            max_log_lines,
            log_scroll_offset: 0,
            active_prompt: None,
            input_prompt_label: String::from("Instruction"),
            quit_armed: false,
            instruction_tx: None,
        }
    }

    pub fn with_input_phase(
        mut self,
        label: String,
        tx: tokio::sync::oneshot::Sender<String>,
    ) -> Self {
        self.phase = TuiPhase::Input {
            buffer: String::new(),
            cursor: 0,
        };
        self.input_prompt_label = label;
        self.instruction_tx = Some(tx);
        self
    }

    pub fn submit_instruction(&mut self) -> Option<String> {
        if let TuiPhase::Input { buffer, .. } = &self.phase {
            let instruction = buffer.clone();
            if let Some(tx) = self.instruction_tx.take() {
                let _ = tx.send(instruction.clone());
            }
            self.phase = TuiPhase::Running;
            return Some(instruction);
        }
        None
    }

    pub fn quit_notice(&self) -> Option<&'static str> {
        self.quit_armed.then_some(CTRL_C_QUIT_MESSAGE)
    }

    fn push_log(&mut self, level: tracing::Level, message: String) {
        if self.logs.len() >= self.max_log_lines {
            self.logs.remove(0);
            if self.log_scroll_offset > 0 {
                self.log_scroll_offset -= 1;
            }
        }

        self.logs.push(LogEntry { level, message });
    }

    fn arm_quit(&mut self) -> KeyAction {
        if self.quit_armed {
            return KeyAction::Quit;
        }

        self.quit_armed = true;
        self.push_log(tracing::Level::WARN, CTRL_C_QUIT_MESSAGE.to_string());
        KeyAction::QuitArmed
    }

    fn clear_quit_notice(&mut self) {
        self.quit_armed = false;
    }

    pub fn handle_event(&mut self, event: TuiEvent) {
        match event {
            TuiEvent::StepStarted {
                task_name,
                task_label,
            } => {
                let existing = self.steps.iter().position(|s| s.task_name == task_name);
                if let Some(idx) = existing {
                    self.steps[idx].task_label = task_label;
                    self.steps[idx].status = StepStatus::Running;
                    self.steps[idx].current_attempt = 0;
                    self.steps[idx].attempt_findings.clear();
                    self.steps[idx].detail_lines.clear();
                } else {
                    self.steps.push(StepState {
                        task_name: task_name.clone(),
                        task_label: task_label.clone(),
                        status: StepStatus::Running,
                        current_attempt: 0,
                        max_attempts: 0,
                        attempt_findings: Vec::new(),
                        detail_lines: vec![format!("started: {task_label}")],
                    });
                }
            }
            TuiEvent::StepAttemptStarted {
                task_name,
                task_label,
                attempt,
            } => {
                if let Some(step) = self.steps.iter_mut().find(|s| s.task_name == task_name) {
                    step.task_label = task_label;
                    step.current_attempt = attempt;
                    step.detail_lines.push(format!("attempt {attempt} started"));
                }
            }
            TuiEvent::StepAttemptValidated {
                task_name,
                task_label,
                attempt,
                accepted,
                finding_count,
            } => {
                if let Some(step) = self.steps.iter_mut().find(|s| s.task_name == task_name) {
                    step.task_label = task_label;
                    step.attempt_findings.push(finding_count);
                    step.detail_lines.push(format!(
                        "attempt {attempt}: accepted={accepted}, findings={finding_count}"
                    ));
                }
            }
            TuiEvent::StepRepairStarted {
                task_name,
                task_label,
                attempt,
            } => {
                if let Some(step) = self.steps.iter_mut().find(|s| s.task_name == task_name) {
                    step.task_label = task_label;
                    step.detail_lines
                        .push(format!("repair planning for attempt {attempt}"));
                }
            }
            TuiEvent::StepCompleted {
                task_name,
                task_label,
                attempts,
            } => {
                if let Some(step) = self.steps.iter_mut().find(|s| s.task_name == task_name) {
                    step.task_label = task_label;
                    step.status = StepStatus::Completed;
                    step.detail_lines
                        .push(format!("completed after {attempts} attempt(s)"));
                }
            }
            TuiEvent::StepRejected {
                task_name,
                task_label,
                attempts,
                reason,
            } => {
                if let Some(step) = self.steps.iter_mut().find(|s| s.task_name == task_name) {
                    step.task_label = task_label;
                    step.status = StepStatus::Rejected {
                        reason: reason.clone(),
                    };
                    step.detail_lines
                        .push(format!("rejected after {attempts} attempt(s): {reason}"));
                }
            }
            TuiEvent::StepFailed {
                task_name,
                task_label,
                stage,
            } => {
                if let Some(step) = self.steps.iter_mut().find(|s| s.task_name == task_name) {
                    step.task_label = task_label;
                    step.status = StepStatus::Failed {
                        stage: stage.clone(),
                    };
                    step.detail_lines.push(format!("failed at stage: {stage}"));
                }
            }
            TuiEvent::ComponentStarted { component, name } => {
                self.push_log(
                    tracing::Level::DEBUG,
                    format!("{component} started: {name}"),
                );
            }
            TuiEvent::ComponentCompleted { component, name } => {
                self.push_log(
                    tracing::Level::DEBUG,
                    format!("{component} completed: {name}"),
                );
            }
            TuiEvent::ComponentFailed { component, name } => {
                self.push_log(tracing::Level::ERROR, format!("{component} failed: {name}"));
            }
            TuiEvent::Log {
                level,
                message,
                target: _,
            } => {
                self.push_log(level, message);
            }
            TuiEvent::HumanPrompt {
                question,
                choices,
                reply,
            } => {
                self.active_prompt = Some(PromptState {
                    question,
                    choices,
                    input: String::new(),
                    reply,
                });
            }
            TuiEvent::Quit => {}
        }
    }

    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> KeyAction {
        if is_ctrl_c(&key) {
            return self.arm_quit();
        }

        if self.quit_armed {
            self.clear_quit_notice();
        }

        if let TuiPhase::Input { buffer, cursor } = &mut self.phase {
            match key.code {
                crossterm::event::KeyCode::Enter => {
                    if let Some(instruction) = self.submit_instruction() {
                        return KeyAction::InstructionSubmitted(instruction);
                    }
                }
                crossterm::event::KeyCode::Char(c) => {
                    buffer.insert(*cursor, c);
                    *cursor += 1;
                }
                crossterm::event::KeyCode::Backspace => {
                    if *cursor > 0 {
                        *cursor -= 1;
                        buffer.remove(*cursor);
                    }
                }
                crossterm::event::KeyCode::Delete => {
                    if *cursor < buffer.len() {
                        buffer.remove(*cursor);
                    }
                }
                crossterm::event::KeyCode::Left => {
                    if *cursor > 0 {
                        *cursor -= 1;
                    }
                }
                crossterm::event::KeyCode::Right => {
                    if *cursor < buffer.len() {
                        *cursor += 1;
                    }
                }
                crossterm::event::KeyCode::Home => {
                    *cursor = 0;
                }
                crossterm::event::KeyCode::End => {
                    *cursor = buffer.len();
                }
                _ => {}
            }
            return KeyAction::Continue;
        }

        if self.active_prompt.is_some() {
            if key.code == crossterm::event::KeyCode::Enter {
                let prompt = self.active_prompt.take().expect("checked is_some");
                let question = prompt.question;
                let reply = prompt.input;
                let _ = prompt.reply.send(reply.clone());
                return KeyAction::PromptSubmitted { question, reply };
            } else if let Some(prompt) = &mut self.active_prompt {
                match key.code {
                    crossterm::event::KeyCode::Char(c) => {
                        prompt.input.push(c);
                    }
                    crossterm::event::KeyCode::Backspace => {
                        prompt.input.pop();
                    }
                    _ => {}
                }
            }
            return KeyAction::Continue;
        }

        match key.code {
            crossterm::event::KeyCode::Up => {
                if self.selected_step > 0 {
                    self.selected_step -= 1;
                }
            }
            crossterm::event::KeyCode::Down => {
                if !self.steps.is_empty() && self.selected_step < self.steps.len() - 1 {
                    self.selected_step += 1;
                }
            }
            crossterm::event::KeyCode::Char('j') => {
                if self.log_scroll_offset > 0 {
                    self.log_scroll_offset -= 1;
                }
            }
            crossterm::event::KeyCode::Char('k') => {
                self.log_scroll_offset += 1;
            }
            crossterm::event::KeyCode::Char('q')
                if key.modifiers.contains(crossterm::event::KeyModifiers::NONE) =>
            {
                return KeyAction::Quit;
            }
            _ => {}
        }
        KeyAction::Continue
    }
}

fn is_ctrl_c(key: &crossterm::event::KeyEvent) -> bool {
    key.modifiers
        .contains(crossterm::event::KeyModifiers::CONTROL)
        && matches!(key.code, crossterm::event::KeyCode::Char('c' | 'C'))
}

pub(crate) fn title_line(state: &AppState) -> Line<'static> {
    let mut spans = vec![Span::styled(
        format!(" {} ", state.title),
        Style::default().add_modifier(Modifier::BOLD),
    )];

    if let Some(notice) = state.quit_notice() {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            notice.to_string(),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
    }

    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use tokio::sync::oneshot;

    use super::{AppState, KeyAction};
    use crate::event::TuiEvent;

    #[test]
    fn enter_submits_input_phase_contents() {
        let (tx, _rx) = oneshot::channel();
        let mut state =
            AppState::new("naaf".to_string(), 100).with_input_phase("Instruction".to_string(), tx);

        assert_eq!(
            state.handle_key(KeyEvent::new(KeyCode::Char('H'), KeyModifiers::NONE)),
            KeyAction::Continue
        );
        assert_eq!(
            state.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE)),
            KeyAction::Continue
        );
        assert_eq!(
            state.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            KeyAction::InstructionSubmitted("Hi".to_string())
        );
    }

    #[test]
    fn enter_submits_human_prompt_reply() {
        let (tx, _rx) = oneshot::channel();
        let mut state = AppState::new("naaf".to_string(), 100);

        state.handle_event(TuiEvent::HumanPrompt {
            question: "Clarify?".to_string(),
            choices: vec!["yes".to_string(), "no".to_string()],
            reply: tx,
        });

        assert_eq!(
            state.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE)),
            KeyAction::Continue
        );
        assert_eq!(
            state.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE)),
            KeyAction::Continue
        );
        assert_eq!(
            state.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE)),
            KeyAction::Continue
        );
        assert_eq!(
            state.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            KeyAction::PromptSubmitted {
                question: "Clarify?".to_string(),
                reply: "yes".to_string(),
            }
        );
    }

    #[test]
    fn ctrl_c_requires_confirmation_before_quitting() {
        let mut state = AppState::new("naaf".to_string(), 100);

        assert_eq!(
            state.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            KeyAction::QuitArmed
        );
        assert_eq!(state.quit_notice(), Some("press ctrl+c again to quit"));
        assert_eq!(
            state.logs.last().map(|entry| entry.message.as_str()),
            state.quit_notice()
        );
        assert_eq!(
            state.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            KeyAction::Quit
        );
    }

    #[test]
    fn ctrl_c_confirmation_clears_after_other_keys() {
        let mut state = AppState::new("naaf".to_string(), 100);

        assert_eq!(
            state.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            KeyAction::QuitArmed
        );
        assert_eq!(
            state.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE)),
            KeyAction::Continue
        );
        assert_eq!(state.quit_notice(), None);
    }
}

pub fn render(frame: &mut Frame, state: &AppState) {
    if let TuiPhase::Input { .. } = &state.phase {
        input::render(frame, state);
        return;
    }

    let size = frame.area();

    let prompt_height = state
        .active_prompt
        .as_ref()
        .map(|prompt| human::desired_height(size.width, prompt))
        .unwrap_or(0);

    let log_height = if state.active_prompt.is_some() { 6 } else { 8 };

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(log_height),
            Constraint::Length(prompt_height),
        ])
        .split(size);

    let title = Paragraph::new(title_line(state));
    frame.render_widget(title, outer[0]);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(outer[1]);

    steps::render(frame, body[0], state);
    detail::render(frame, body[1], state);
    logs::render(frame, outer[2], state);

    if state.active_prompt.is_some() {
        human::render(frame, outer[3], state);
    }
}
