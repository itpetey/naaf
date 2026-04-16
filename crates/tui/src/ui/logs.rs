use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::ui::AppState;

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let visible_height = area.height.saturating_sub(2) as usize;

    let total = state.logs.len();
    let max_scroll = total.saturating_sub(visible_height);
    let scroll = state.log_scroll_offset.min(max_scroll);
    let start = total.saturating_sub(visible_height + scroll);

    let lines: Vec<Line> = state
        .logs
        .iter()
        .skip(start)
        .take(visible_height)
        .map(|entry| {
            let color = match entry.level {
                tracing::Level::ERROR => Color::Red,
                tracing::Level::WARN => Color::Yellow,
                tracing::Level::INFO => Color::Cyan,
                tracing::Level::DEBUG => Color::DarkGray,
                tracing::Level::TRACE => Color::DarkGray,
            };
            Line::from(vec![
                Span::styled(format!("{:<5} ", entry.level), Style::default().fg(color)),
                Span::raw(&entry.message),
            ])
        })
        .collect();

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(Span::styled(
            " Logs ",
            Style::default().add_modifier(ratatui::style::Modifier::BOLD),
        )))
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}
