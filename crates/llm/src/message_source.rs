use std::sync::Arc;

/// A source of deferred user messages that can be injected between LLM turns.
///
/// When an [`Executor`](crate::Executor) is configured with a message source,
/// it drains pending messages after each batch of tool calls and appends them
/// as [`Message::user`] entries before the next turn, ensuring queued user
/// input reaches the LLM at the earliest possible opportunity.
pub trait MessageSource: Send + Sync {
    /// Drain all pending messages, returning them in the order they were queued.
    fn drain_messages(&self) -> Vec<String>;
}

impl MessageSource for Arc<dyn MessageSource> {
    fn drain_messages(&self) -> Vec<String> {
        (**self).drain_messages()
    }
}

impl MessageSource for std::sync::Mutex<Vec<String>> {
    fn drain_messages(&self) -> Vec<String> {
        let mut guard = self.lock().expect("message source lock poisoned");
        std::mem::take(&mut *guard)
    }
}
