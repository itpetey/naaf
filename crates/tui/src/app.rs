use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{Mutex, mpsc, oneshot};
use tracing_subscriber::prelude::*;

use crate::event::TuiEvent;
use crate::terminal::TerminalGuard;
use crate::ui::AppState;

const DEFAULT_TICK_MS: u64 = 100;
const DEFAULT_MAX_LOG_LINES: usize = 1000;

pub type EventSender = mpsc::UnboundedSender<TuiEvent>;
pub type InstructionReceiver = oneshot::Receiver<String>;

pub struct TuiAppBuilder {
    title: String,
    tick_rate: Duration,
    max_log_lines: usize,
    install_tracing_layer: bool,
    input_screen: Option<String>,
}

impl TuiAppBuilder {
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn tick_rate(mut self, duration: Duration) -> Self {
        self.tick_rate = duration;
        self
    }

    pub fn max_log_lines(mut self, n: usize) -> Self {
        self.max_log_lines = n;
        self
    }

    pub fn install_tracing_layer(mut self) -> Self {
        self.install_tracing_layer = true;
        self
    }

    pub fn with_input_screen(mut self, label: impl Into<String>) -> Self {
        self.input_screen = Some(label.into());
        self
    }

    pub fn spawn(self) -> Result<(EventSender, TuiHandle), TuiError> {
        let (tx, rx) = mpsc::unbounded_channel();

        if self.install_tracing_layer {
            let layer = crate::tracing_layer::TuiLayer::new(tx.clone());
            tracing_subscriber::registry().with(layer).init();
        }

        let title = self.title;
        let tick_rate = self.tick_rate;
        let max_log_lines = self.max_log_lines;
        let input_label = self.input_screen.clone();

        let join_handle = Arc::new(Mutex::new(Some(tokio::spawn(async move {
            let _ = run_app(rx, title, tick_rate, max_log_lines, input_label, None).await;
        }))));

        let handle = TuiHandle { join_handle };

        Ok((tx, handle))
    }

    pub fn spawn_with_input(
        self,
    ) -> Result<(EventSender, TuiHandle, InstructionReceiver), TuiError> {
        let (instruction_tx, instruction_rx) = oneshot::channel();

        let (tx, rx) = mpsc::unbounded_channel();

        if self.install_tracing_layer {
            let layer = crate::tracing_layer::TuiLayer::new(tx.clone());
            tracing_subscriber::registry().with(layer).init();
        }

        let title = self.title;
        let tick_rate = self.tick_rate;
        let max_log_lines = self.max_log_lines;
        let input_label = self
            .input_screen
            .clone()
            .unwrap_or_else(|| "Instruction".to_string());

        let join_handle = Arc::new(Mutex::new(Some(tokio::spawn(async move {
            let _ = run_app(
                rx,
                title,
                tick_rate,
                max_log_lines,
                Some(input_label),
                Some(instruction_tx),
            )
            .await;
        }))));

        let handle = TuiHandle { join_handle };

        Ok((tx, handle, instruction_rx))
    }
}

impl Default for TuiAppBuilder {
    fn default() -> Self {
        Self {
            title: String::from("naaf"),
            tick_rate: Duration::from_millis(DEFAULT_TICK_MS),
            max_log_lines: DEFAULT_MAX_LOG_LINES,
            install_tracing_layer: false,
            input_screen: None,
        }
    }
}

pub struct TuiHandle {
    join_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl TuiHandle {
    pub async fn shutdown(self) -> Result<(), TuiError> {
        let handle = {
            let mut guard = self.join_handle.lock().await;
            guard.take()
        };
        if let Some(h) = handle {
            h.await.map_err(|e| TuiError::Terminal(e.to_string()))?;
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TuiError {
    #[error("terminal error: {0}")]
    Terminal(String),
}

async fn run_app(
    mut rx: mpsc::UnboundedReceiver<TuiEvent>,
    title: String,
    tick_rate: Duration,
    max_log_lines: usize,
    input_label: Option<String>,
    instruction_tx: Option<oneshot::Sender<String>>,
) -> Result<(), TuiError> {
    let mut terminal = TerminalGuard::new().map_err(|e| TuiError::Terminal(e.to_string()))?;

    let mut app_state = AppState::new(title, max_log_lines);

    if let (Some(label), Some(tx)) = (input_label, instruction_tx) {
        app_state = app_state.with_input_phase(label, tx);
    }

    loop {
        while let Ok(event) = rx.try_recv() {
            if matches!(event, TuiEvent::Quit) {
                return Ok(());
            }
            app_state.handle_event(event);
        }

        terminal
            .draw(|frame| crate::ui::render(frame, &app_state))
            .map_err(|e| TuiError::Terminal(e.to_string()))?;

        if crossterm::event::poll(tick_rate).map_err(|e| TuiError::Terminal(e.to_string()))?
            && let Ok(event) = crossterm::event::read()
            && let crossterm::event::Event::Key(key) = event
        {
            if key.code == crossterm::event::KeyCode::Char('q')
                && key.modifiers.contains(crossterm::event::KeyModifiers::NONE)
            {
                return Ok(());
            }
            app_state.handle_key(key);
        }
    }
}
