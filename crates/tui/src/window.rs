use crossterm::event::{self, Event, KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, BorderType, List, ListItem, Paragraph, Wrap},
};
use std::io;

pub struct TuiApp {
    pub selected_workflow: Option<usize>,
    pub workflows: Vec<String>,
    pub messages: Vec<String>,
    pub current_view: View,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    WorkflowList,
    Execution,
    Inspect,
}

impl TuiApp {
    pub fn new(workflows: Vec<String>) -> Self {
        Self {
            selected_workflow: None,
            workflows,
            messages: Vec::new(),
            current_view: View::WorkflowList,
        }
    }

    pub fn add_message(&mut self, message: String) {
        self.messages.push(message);
        if self.messages.len() > 100 {
            self.messages.remove(0);
        }
    }

    pub fn select_next(&mut self) {
        if let Some(selected) = self.selected_workflow {
            if selected < self.workflows.len() - 1 {
                self.selected_workflow = Some(selected + 1);
            }
        } else if !self.workflows.is_empty() {
            self.selected_workflow = Some(0);
        }
    }

    pub fn select_previous(&mut self) {
        if let Some(selected) = self.selected_workflow {
            if selected > 0 {
                self.selected_workflow = Some(selected - 1);
            }
        } else if !self.workflows.is_empty() {
            self.selected_workflow = Some(self.workflows.len() - 1);
        }
    }

    pub fn render(&self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(f.area());

        let title = Paragraph::new("NAAF TUI")
            .block(Block::bordered().border_type(BorderType::Rounded))
            .centered();
        f.render_widget(title, chunks[0]);

        match self.current_view {
            View::WorkflowList => self.render_workflow_list(f, &chunks[1]),
            View::Execution => self.render_execution(f, &chunks[1]),
            View::Inspect => self.render_inspect(f, &chunks[1]),
        }

        let status_text = match self.current_view {
            View::WorkflowList => "↑/↓: Select | Enter: Run | Esc: Quit",
            View::Execution => "Ctrl+C: Abort | Esc: Back",
            View::Inspect => "↑/↓: Scroll | Esc: Back",
        };
        let status =
            Paragraph::new(status_text).block(Block::bordered().border_type(BorderType::Rounded));
        f.render_widget(status, chunks[2]);
    }

    fn render_workflow_list(&self, f: &mut Frame, area: &Rect) {
        let items: Vec<ListItem> = self
            .workflows
            .iter()
            .enumerate()
            .map(|(i, workflow)| {
                if Some(i) == self.selected_workflow {
                    ListItem::new(workflow.as_str()).style(Style::default().bg(Color::Blue))
                } else {
                    ListItem::new(workflow.as_str())
                }
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::bordered()
                    .title("Workflows")
                    .border_type(BorderType::Rounded),
            )
            .highlight_style(Style::default().bg(Color::Blue));

        f.render_widget(list, *area);
    }

    fn render_execution(&self, f: &mut Frame, area: &Rect) {
        let content = if self.messages.is_empty() {
            "Executing workflow..."
        } else {
            self.messages
                .last()
                .map(|s| s.as_str())
                .unwrap_or("Executing workflow...")
        };

        let paragraph = Paragraph::new(content)
            .block(
                Block::bordered()
                    .title("Execution")
                    .border_type(BorderType::Rounded),
            )
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, *area);
    }

    fn render_inspect(&self, f: &mut Frame, area: &Rect) {
        let content = if self.messages.is_empty() {
            "No execution data"
        } else {
            &self.messages.join("\n")
        };

        let paragraph = Paragraph::new(content)
            .block(
                Block::bordered()
                    .title("Run Inspector")
                    .border_type(BorderType::Rounded),
            )
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, *area);
    }
}

pub fn spawn_tui_window(workflows: Vec<String>) -> io::Result<()> {
    use ratatui::Terminal;
    use ratatui::backend::CrosstermBackend;

    let mut app = TuiApp::new(workflows);

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    loop {
        terminal.draw(|f| app.render(f))?;

        match app.current_view {
            View::WorkflowList => {
                if let Ok(Event::Key(key)) = event::read() {
                    match key {
                        KeyEvent {
                            code: KeyCode::Down,
                            ..
                        } => app.select_next(),
                        KeyEvent {
                            code: KeyCode::Up, ..
                        } => app.select_previous(),
                        KeyEvent {
                            code: KeyCode::Enter,
                            ..
                        } => {
                            if app.selected_workflow.is_some() {
                                app.current_view = View::Execution;
                            }
                        }
                        KeyEvent {
                            code: KeyCode::Esc, ..
                        } => break,
                        _ => {}
                    }
                }
            }
            View::Execution => {
                if let Ok(Event::Key(KeyEvent {
                    code: KeyCode::Esc, ..
                })) = event::read()
                {
                    app.current_view = View::WorkflowList;
                }
            }
            View::Inspect => {
                if let Ok(Event::Key(KeyEvent {
                    code: KeyCode::Esc, ..
                })) = event::read()
                {
                    app.current_view = View::WorkflowList;
                }
            }
        }
    }

    Ok(())
}
