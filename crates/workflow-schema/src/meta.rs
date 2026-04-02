use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StateMeta {
    pub created_at: DateTime<Utc>,
}

impl StateMeta {
    pub fn now() -> Self {
        Self {
            created_at: Utc::now(),
        }
    }
}
