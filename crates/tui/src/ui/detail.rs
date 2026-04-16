use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::ui::{AppState, StepStatus};

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let step = state.steps.get(state.selected_step);

    let lines = if let Some(step) = step {
        let status_line = match &step.status {
            StepStatus::Running => Line::from(Span::styled(
                format!(
                    "[running] {} (attempt {})",
                    step.task_name, step.current_attempt
                ),
                Style::default().fg(Color::Yellow),
            )),
            StepStatus::Completed => Line::from(Span::styled(
                format!("[completed] {}", step.task_name),
                Style::default().fg(Color::Green),
            )),
            StepStatus::Rejected { reason } => Line::from(vec![
                Span::styled(
                    format!("[rejected] {} ", step.task_name),
                    Style::default().fg(Color::Red),
                ),
                Span::styled(reason.clone(), Style::default().fg(Color::LightRed)),
            ]),
            StepStatus::Failed { stage } => Line::from(vec![
                Span::styled(
                    format!("[failed] {} ", step.task_name),
                    Style::default().fg(Color::Red),
                ),
                Span::styled(stage.clone(), Style::default().fg(Color::LightRed)),
            ]),
        };

        let mut detail = vec![status_line, Line::default()];

        for line in &step.detail_lines {
            detail.push(Line::from(Span::raw(line.clone())));
        }

        if !step.attempt_findings.is_empty() {
            detail.push(Line::default());
            detail.push(Line::from(Span::styled(
                "Findings per attempt:",
                Style::default().add_modifier(Modifier::BOLD),
            )));
            for (i, count) in step.attempt_findings.iter().enumerate() {
                detail.push(Line::from(format!(
                    "  attempt {}: {} finding(s)",
                    i + 1,
                    count
                )));
            }
        }

        detail
    } else {
        vec![Line::from(Span::styled(
            "No step selected",
            Style::default().fg(Color::DarkGray),
        ))]
    };

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(Span::styled(
            " Detail ",
            Style::default().add_modifier(Modifier::BOLD),
        )))
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}
