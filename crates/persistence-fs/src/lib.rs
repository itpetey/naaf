//! Filesystem-based checkpoint persistence for `naaf`.
//!
//! `naaf-persistence-fs` provides a `Checkpointer` implementation that stores
//! workflow and step checkpoints as JSON files on the local filesystem.
//!
//! # Usage
//!
//! ```ignore
//! use naaf_core::{Checkpointer, WorkflowRunId, NodeId};
//! use naaf_persistence_fs::FsCheckpointer;
//!
//! let checkpointer = FsCheckpointer::new("/tmp/naaf-checkpoints");
//! ```
//!
//! The checkpointer organizes checkpoints by workflow run ID:
//! - `/base_dir/{run_id}/workflow.json` — workflow-level checkpoint
//! - `/base_dir/{run_id}/steps/{node_id}.json` — per-step checkpoints

use std::path::PathBuf;

use naaf_core::{NodeId, StepCheckpoint, WorkflowCheckpoint, WorkflowRunId};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FsCheckpointerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialisation error: {0}")]
    Serialisation(#[from] serde_json::Error),
}

pub struct FsCheckpointer {
    base_dir: PathBuf,
}

impl FsCheckpointer {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    fn workflow_dir(&self, run_id: WorkflowRunId) -> PathBuf {
        self.base_dir.join(run_id.to_string())
    }

    fn workflow_path(&self, run_id: WorkflowRunId) -> PathBuf {
        self.workflow_dir(run_id).join("workflow.json")
    }

    fn step_path(&self, run_id: WorkflowRunId, node_id: NodeId) -> PathBuf {
        self.workflow_dir(run_id)
            .join("steps")
            .join(format!("{node_id}.json"))
    }
}

impl naaf_core::Checkpointer for FsCheckpointer {
    fn save_workflow(
        &self,
        run_id: WorkflowRunId,
        checkpoint: &WorkflowCheckpoint,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>,
                > + Send,
        >,
    > {
        let path = self.workflow_path(run_id);
        let dir = self.workflow_dir(run_id);
        let data = serde_json::to_string_pretty(checkpoint);
        Box::pin(async move {
            let data = data?;
            tokio::fs::create_dir_all(&dir).await?;
            tokio::fs::write(&path, data).await?;
            Ok(())
        })
    }

    fn load_workflow(
        &self,
        run_id: WorkflowRunId,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        Option<WorkflowCheckpoint>,
                        Box<dyn std::error::Error + Send + Sync + 'static>,
                    >,
                > + Send,
        >,
    > {
        let path = self.workflow_path(run_id);
        Box::pin(async move {
            let data = match tokio::fs::read_to_string(&path).await {
                Ok(data) => data,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                Err(error) => {
                    return Err(Box::new(error) as Box<dyn std::error::Error + Send + Sync>);
                }
            };
            let checkpoint: WorkflowCheckpoint = serde_json::from_str(&data)?;
            Ok(Some(checkpoint))
        })
    }

    fn save_step(
        &self,
        run_id: WorkflowRunId,
        node_id: NodeId,
        checkpoint: &StepCheckpoint,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>,
                > + Send,
        >,
    > {
        let path = self.step_path(run_id, node_id);
        let data = serde_json::to_string_pretty(checkpoint);
        Box::pin(async move {
            let data = data?;
            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::write(&path, data).await?;
            Ok(())
        })
    }

    fn load_step(
        &self,
        run_id: WorkflowRunId,
        node_id: NodeId,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        Option<StepCheckpoint>,
                        Box<dyn std::error::Error + Send + Sync + 'static>,
                    >,
                > + Send,
        >,
    > {
        let path = self.step_path(run_id, node_id);
        Box::pin(async move {
            let data = match tokio::fs::read_to_string(&path).await {
                Ok(data) => data,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                Err(error) => {
                    return Err(Box::new(error) as Box<dyn std::error::Error + Send + Sync>);
                }
            };
            let checkpoint: StepCheckpoint = serde_json::from_str(&data)?;
            Ok(Some(checkpoint))
        })
    }

    fn delete_workflow(
        &self,
        run_id: WorkflowRunId,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>,
                > + Send,
        >,
    > {
        let dir = self.workflow_dir(run_id);
        Box::pin(async move {
            let result = tokio::fs::remove_dir_all(&dir).await;
            match result {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(Box::new(error) as Box<dyn std::error::Error + Send + Sync>),
            }
        })
    }
}
