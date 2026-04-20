//! WebSocket front end for `naaf` workflows.
//!
//! `naaf-ws` exposes workflow progress, structured logs, and prompt requests to
//! browser clients over a WebSocket connection.

mod event;
mod frontend_event;
mod layer;
mod server;

pub use event::{WsClientMessage, WsEvent};
pub use frontend_event::FrontendEvent;
pub use layer::WsLayer;
pub use server::{EventSender, InstructionReceiver, WsAppBuilder, WsError, WsHandle};
