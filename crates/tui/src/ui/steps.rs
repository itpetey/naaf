use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};

use crate::ui::{AppState, StepStatus};

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let items: Vec<ListItem> = state
        .steps
        .iter()
        .map(|step| {
            let (icon, color) = match &step.status {
                StepStatus::Running => ("\u{27F3}", Color::Yellow),
                StepStatus::Completed => ("\u{2713}", Color::Green),
                StepStatus::Rejected { .. } => ("\u{2717}", Color::Red),
                StepStatus::Failed { .. } => ("\u{2717}", Color::Red),
            };

            let attempt_info = if step.current_attempt > 0 {
                format!(" {}/{}", step.current_attempt, step.max_attempts)
            } else {
                String::new()
            };

            let line = Line::from(vec![
                Span::styled(format!("{icon} "), Style::default().fg(color)),
                Span::styled(
                    format!("{}{}", step.task_name, attempt_info),
                    Style::default(),
                ),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(Span::styled(
        " Steps ",
        Style::default().add_modifier(Modifier::BOLD),
    )));

    let mut list_state = ListState::default();
    if !state.steps.is_empty() {
        list_state.select(Some(state.selected_step));
    }

    frame.render_stateful_widget(list, area, &mut list_state);
}
