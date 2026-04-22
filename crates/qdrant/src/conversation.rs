use serde::{Deserialize, Serialize};

/// One message in a serialised conversation transcript.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    /// Speaker role, such as `user` or `assistant`.
    pub role: String,
    /// Message content for the given speaker.
    pub content: String,
}
