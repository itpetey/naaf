//! Finding types for validation and review results.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FindingId(pub Uuid);

impl FindingId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for FindingId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for FindingId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FindingStatus {
    #[default]
    Open,
    Resolved,
    Regressed,
    Deferred,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: FindingId,
    pub run_id: super::run::RunId,
    pub source: String,
    pub severity: Severity,
    pub category: String,
    pub status: FindingStatus,
    pub evidence: Vec<String>,
    pub affected_paths: Vec<PathBuf>,
    pub suggested_fix_scope: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

impl Finding {
    pub fn new(
        run_id: super::run::RunId,
        source: String,
        severity: Severity,
        category: String,
        evidence: Vec<String>,
        affected_paths: Vec<PathBuf>,
    ) -> Self {
        Self {
            id: FindingId::new(),
            run_id,
            source,
            severity,
            category,
            status: FindingStatus::default(),
            evidence,
            affected_paths,
            suggested_fix_scope: Vec::new(),
            created_at: Utc::now(),
            resolved_at: None,
        }
    }

    pub fn resolve(&mut self) {
        self.status = FindingStatus::Resolved;
        self.resolved_at = Some(Utc::now());
    }

    pub fn mark_regressed(&mut self) {
        self.status = FindingStatus::Regressed;
    }
}
