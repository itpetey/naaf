use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::ui::AppState;

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let Some(prompt) = &state.active_prompt else {
        return;
    };

    let mut lines = vec![Line::from(Span::styled(
        format!("? {}", prompt.question),
        Style::default().fg(Color::Yellow),
    ))];

    if !prompt.choices.is_empty() {
        let choices_text = prompt.choices.join(" / ");
        lines.push(Line::from(Span::styled(
            format!("  [{choices_text}]"),
            Style::default().fg(Color::DarkGray),
        )));
    }

    lines.push(Line::from(vec![
        Span::raw("> "),
        Span::styled(prompt.input.clone(), Style::default().fg(Color::White)),
        Span::styled("\u{2588}", Style::default().fg(Color::White)),
    ]));

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );

    frame.render_widget(paragraph, area);
}
