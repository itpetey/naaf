use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StateMeta {
    pub created_at: DateTime<Utc>,
    pub token_count: Option<u64>,
}

impl StateMeta {
    pub fn now() -> Self {
        Self {
            created_at: Utc::now(),
            token_count: None,
        }
    }

    pub fn with_tokens(token_count: u64) -> Self {
        Self {
            created_at: Utc::now(),
            token_count: Some(token_count),
        }
    }
}
