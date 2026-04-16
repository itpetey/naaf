use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::ui::{AppState, TuiPhase};

pub fn render(frame: &mut Frame, state: &AppState) {
    let size = frame.area();

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .split(size);

    let title = Paragraph::new(Span::styled(
        format!(" {} ", state.title),
        Style::default().add_modifier(Modifier::BOLD),
    ));
    frame.render_widget(title, outer[0]);

    let TuiPhase::Input { buffer, cursor } = &state.phase else {
        return;
    };

    let display_before = buffer[..*cursor].to_string();
    let display_after = buffer[*cursor..].to_string();

    let input_line = Line::from(vec![
        Span::raw(display_before),
        Span::styled("\u{2588}", Style::default().fg(Color::Cyan)),
        Span::raw(display_after),
    ]);

    let input = Paragraph::new(vec![input_line]).block(
        Block::default()
            .borders(Borders::ALL)
            .title(Span::styled(
                format!(" {} ", state.input_prompt_label),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ))
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(input, outer[1]);

    let help = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "Enter your instruction and press Enter to begin.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Navigation:",
            Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::raw("  Enter      Submit instruction")),
        Line::from(Span::raw("  \u{2190}/\u{2192}       Move cursor")),
        Line::from(Span::raw("  Home/End   Jump to start/end")),
        Line::from(Span::raw("  q          Quit")),
    ])
    .wrap(Wrap { trim: false });
    frame.render_widget(help, outer[2]);
}
