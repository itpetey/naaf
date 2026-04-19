use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use unicode_width::UnicodeWidthStr;

use crate::ui::{AppState, PromptState};

const MIN_PROMPT_HEIGHT: u16 = 6;

pub fn desired_height(width: u16, prompt: &PromptState) -> u16 {
    let content_width = width.saturating_sub(4).max(1) as usize;
    let mut line_count = wrap_with_prefix(&prompt.question, content_width, "? ", "  ").len() as u16;

    if !prompt.choices.is_empty() {
        line_count += wrap_with_prefix(
            &prompt.choices.join(" / "),
            content_width,
            "Options: ",
            "         ",
        )
        .len() as u16;
    }

    // Border, a spacer before the input row, and a short submit hint.
    (line_count + 5).max(MIN_PROMPT_HEIGHT)
}

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let Some(prompt) = &state.active_prompt else {
        return;
    };

    let content_width = area.width.saturating_sub(4).max(1) as usize;
    let mut lines: Vec<Line> = wrap_with_prefix(&prompt.question, content_width, "? ", "  ")
        .into_iter()
        .map(|line| Line::from(Span::styled(line, Style::default().fg(Color::Yellow))))
        .collect();

    if !prompt.choices.is_empty() {
        let choices_lines = wrap_with_prefix(
            &prompt.choices.join(" / "),
            content_width,
            "Options: ",
            "         ",
        );

        lines.extend(
            choices_lines
                .into_iter()
                .map(|line| Line::from(Span::styled(line, Style::default().fg(Color::DarkGray)))),
        );
    }

    lines.push(Line::default());
    lines.push(Line::from(vec![
        Span::raw("> "),
        Span::styled(prompt.input.clone(), Style::default().fg(Color::White)),
        Span::styled("\u{2588}", Style::default().fg(Color::White)),
    ]));
    lines.push(Line::from(Span::styled(
        "Press Enter to submit",
        Style::default().fg(Color::DarkGray),
    )));

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false }).block(
        Block::default()
            .borders(Borders::ALL)
            .title(Span::styled(
                " Action Required ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ))
            .border_style(Style::default().fg(Color::Yellow)),
    );

    frame.render_widget(paragraph, area);
}

fn wrap_with_prefix(
    text: &str,
    width: usize,
    first_prefix: &str,
    rest_prefix: &str,
) -> Vec<String> {
    if width == 0 {
        return Vec::new();
    }

    let mut lines = Vec::new();

    for raw_line in text.lines() {
        let source = if raw_line.trim().is_empty() {
            " "
        } else {
            raw_line
        };
        let mut words = source.split_whitespace();
        let mut current = String::new();
        let mut current_width = 0;
        let mut prefix = first_prefix;

        let prefix_width = UnicodeWidthStr::width(prefix);
        let available_width = width.saturating_sub(prefix_width).max(1);

        for word in words.by_ref() {
            let word_width = UnicodeWidthStr::width(word);
            let needed_width = if current.is_empty() {
                word_width
            } else {
                current_width + 1 + word_width
            };

            if needed_width <= available_width || current.is_empty() {
                if current.is_empty() {
                    current.push_str(word);
                    current_width = word_width;
                } else {
                    current.push(' ');
                    current.push_str(word);
                    current_width = needed_width;
                }
                continue;
            }

            lines.push(format!("{prefix}{current}"));
            prefix = rest_prefix;
            current.clear();
            current.push_str(word);
            current_width = word_width;
        }

        if current.is_empty() {
            lines.push(prefix.trim_end().to_string());
        } else {
            lines.push(format!("{prefix}{current}"));
        }
    }

    if lines.is_empty() {
        lines.push(first_prefix.trim_end().to_string());
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::{desired_height, wrap_with_prefix};
    use crate::ui::PromptState;

    #[test]
    fn wraps_multiline_prompt_text() {
        let wrapped = wrap_with_prefix(
            "Proposal: A very long title that should wrap\nReply with approve to continue",
            24,
            "? ",
            "  ",
        );

        assert_eq!(wrapped.first().unwrap(), "? Proposal: A very long");
        assert!(
            wrapped
                .iter()
                .any(|line| line == "  title that should wrap")
        );
        assert!(wrapped.iter().any(|line| line == "? Reply with approve to"));
    }

    #[test]
    fn prompt_height_grows_for_long_questions() {
        let short_prompt = PromptState {
            question: "Proposal: Short title".to_string(),
            choices: Vec::new(),
            input: String::new(),
            reply: tokio::sync::oneshot::channel().0,
        };
        let long_prompt = PromptState {
            question:
                "Proposal: A much longer title that needs several terminal lines to remain readable"
                    .to_string(),
            choices: vec!["approve".to_string(), "revise".to_string()],
            input: String::new(),
            reply: tokio::sync::oneshot::channel().0,
        };

        assert!(desired_height(32, &long_prompt) > desired_height(80, &short_prompt));
    }
}
