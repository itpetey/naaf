use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use unicode_width::UnicodeWidthStr;

use crate::ui::{AppState, StepStatus};

fn wrap_step(icon: &str, icon_color: Color, text: &str, available_width: u16) -> ListItem<'static> {
    let prefix = format!("{icon} ");
    let prefix_width = UnicodeWidthStr::width(prefix.as_str());
    let indent = " ".repeat(prefix_width);
    let max_content = available_width.saturating_sub(prefix_width as u16) as usize;

    if max_content == 0 || text.is_empty() {
        return ListItem::new(Line::from(Span::styled(
            prefix,
            Style::default().fg(icon_color),
        )));
    }

    let words: Vec<&str> = text.split_whitespace().collect();
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current = String::new();
    let mut current_width = 0;
    let mut is_first = true;

    for word in &words {
        let word_width = UnicodeWidthStr::width(*word);
        let needed = if current.is_empty() {
            word_width
        } else {
            current_width + 1 + word_width
        };

        if needed <= max_content {
            if current.is_empty() {
                current = word.to_string();
                current_width = word_width;
            } else {
                current.push(' ');
                current.push_str(word);
                current_width = needed;
            }
        } else if current.is_empty() {
            current = word.to_string();
            current_width = word_width;
        } else {
            if is_first {
                lines.push(Line::from(vec![
                    Span::styled(prefix.clone(), Style::default().fg(icon_color)),
                    Span::raw(current),
                ]));
                is_first = false;
            } else {
                lines.push(Line::from(vec![
                    Span::raw(indent.clone()),
                    Span::raw(current),
                ]));
            }
            current = word.to_string();
            current_width = word_width;
        }
    }

    if !current.is_empty() {
        if is_first {
            lines.push(Line::from(vec![
                Span::styled(prefix, Style::default().fg(icon_color)),
                Span::raw(current),
            ]));
        } else {
            lines.push(Line::from(vec![Span::raw(indent), Span::raw(current)]));
        }
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("{icon} "),
            Style::default().fg(icon_color),
        )));
    }

    ListItem::new(Text::from(lines))
}

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let content_width = area.width.saturating_sub(2);

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

            let text = format!("{}{}", step.task_label, attempt_info);
            wrap_step(icon, color, &text, content_width)
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
