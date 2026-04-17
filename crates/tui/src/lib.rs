//! Terminal user interface for `naaf` workflows.
//!
//! `naaf-tui` provides a terminal UI for observing and interacting with naaf
//! workflows. It displays step progress, logs, and findings in real-time,
//! and can prompt the user for input when a workflow requires human feedback.
//!
//! # Features
//!
//! - Real-time step progress and status display
//! - Structured log viewer with filtering
//! - Human-in-the-loop prompting for workflows that need user input
//! - Integration with `naaf_core` observability via tracing layer

pub use app::{EventSender, InstructionReceiver, TuiAppBuilder, TuiHandle};
pub use event::TuiEvent;
pub use tracing_layer::TuiLayer;

mod app;
mod event;
mod terminal;
mod tracing_layer;
mod ui;
