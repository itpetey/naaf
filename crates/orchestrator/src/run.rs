//! Run lifecycle and execution state.
//!
//! # Legacy Code
//!
//! This module is part of the legacy prototype runtime.
//! **Do not build new features on this code.**
//! See the repository root `LEGACY.md` for details.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

pub use naaf_openspec::Phase;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskId(pub Uuid);

impl TaskId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunId(pub Uuid);

impl RunId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for RunId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for RunId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TerminalReason {
    Escalated { message: String },
    Failed { message: String },
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum Outcome {
    #[default]
    InProgress,
    Done,
    Escalated(TerminalReason),
    Failed(TerminalReason),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub prompt: String,
    pub repo: Option<PathBuf>,
    pub created_at: DateTime<Utc>,
}

impl Task {
    pub fn new(prompt: String, repo: Option<PathBuf>) -> Self {
        Self {
            id: TaskId::new(),
            prompt,
            repo,
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    pub id: RunId,
    pub task_id: TaskId,
    pub phase: Phase,
    pub outcome: Outcome,
    pub worktree: PathBuf,
    pub head: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Run {
    #[tracing::instrument(skip_all)]
    pub fn new(task_id: TaskId, worktree: PathBuf) -> Self {
        let now = Utc::now();
        Self {
            id: RunId::new(),
            task_id,
            phase: Phase::default(),
            outcome: Outcome::default(),
            worktree,
            head: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn transition_to(&mut self, phase: Phase) {
        self.phase = phase;
        self.updated_at = Utc::now();
    }

    pub fn complete(&mut self) {
        self.outcome = Outcome::Done;
        self.updated_at = Utc::now();
    }

    pub fn fail(&mut self, reason: TerminalReason) {
        self.outcome = match reason {
            r @ TerminalReason::Escalated { .. } => Outcome::Escalated(r),
            r @ TerminalReason::Failed { .. } => Outcome::Failed(r),
            r @ TerminalReason::Cancelled => Outcome::Failed(r),
        };
        self.updated_at = Utc::now();
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self.outcome,
            Outcome::Done | Outcome::Escalated(..) | Outcome::Failed(..)
        )
    }
}
