//! SQLite-based checkpoint persistence for `naaf`.
//!
//! `naaf-persistence-sqlite` provides a `Checkpointer` implementation that stores
//! workflow and step checkpoints in a SQLite database.
//!
//! # Usage
//!
//! ```ignore
//! use naaf_core::Checkpointer;
//! use naaf_persistence_sqlite::SqliteCheckpointer;
//!
//! let checkpointer = SqliteCheckpointer::open("checkpoints.db")?;
//! ```
//!
//! The checkpointer uses two tables:
//! - `workflow_checkpoints` — stores workflow-level checkpoints by run ID
//! - `step_checkpoints` — stores per-step checkpoints by run ID and node

use std::sync::Arc;

use naaf_core::{NodeId, StepCheckpoint, WorkflowCheckpoint, WorkflowRunId};
use parking_lot::Mutex;
use rusqlite::{Connection, params};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SqliteCheckpointerError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("serialisation error: {0}")]
    Serialisation(#[from] serde_json::Error),
}

pub struct SqliteCheckpointer {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteCheckpointer {
    pub fn new(conn: Connection) -> Result<Self, SqliteCheckpointerError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS workflow_checkpoints (
                run_id TEXT PRIMARY KEY,
                data TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS step_checkpoints (
                run_id TEXT NOT NULL,
                node_id TEXT NOT NULL,
                data TEXT NOT NULL,
                PRIMARY KEY (run_id, node_id)
            );",
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn open(path: &str) -> Result<Self, SqliteCheckpointerError> {
        let conn = Connection::open(path)?;
        Self::new(conn)
    }

    pub fn open_in_memory() -> Result<Self, SqliteCheckpointerError> {
        let conn = Connection::open_in_memory()?;
        Self::new(conn)
    }
}

impl naaf_core::Checkpointer for SqliteCheckpointer {
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
        let conn = self.conn.clone();
        let run_id_str = run_id.to_string();
        let data = serde_json::to_string(checkpoint);
        Box::pin(async move {
            let data = data?;
            let conn = conn.lock();
            conn.execute(
                "INSERT OR REPLACE INTO workflow_checkpoints (run_id, data) VALUES (?1, ?2)",
                params![run_id_str, data],
            )?;
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
        let conn = self.conn.clone();
        let run_id_str = run_id.to_string();
        Box::pin(async move {
            let conn = conn.lock();
            let mut stmt =
                conn.prepare("SELECT data FROM workflow_checkpoints WHERE run_id = ?1")?;
            let result = stmt.query_row(params![run_id_str], |row| {
                let data: String = row.get(0)?;
                Ok(data)
            });
            match result {
                Ok(data) => {
                    let checkpoint: WorkflowCheckpoint = serde_json::from_str(&data)?;
                    Ok(Some(checkpoint))
                }
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(error) => Err(Box::new(error) as Box<dyn std::error::Error + Send + Sync>),
            }
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
        let conn = self.conn.clone();
        let run_id_str = run_id.to_string();
        let node_id_str = node_id.to_string();
        let data = serde_json::to_string(checkpoint);
        Box::pin(async move {
            let data = data?;
            let conn = conn.lock();
            conn.execute(
                "INSERT OR REPLACE INTO step_checkpoints (run_id, node_id, data) VALUES (?1, ?2, ?3)",
                params![run_id_str, node_id_str, data],
            )?;
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
        let conn = self.conn.clone();
        let run_id_str = run_id.to_string();
        let node_id_str = node_id.to_string();
        Box::pin(async move {
            let conn = conn.lock();
            let mut stmt = conn
                .prepare("SELECT data FROM step_checkpoints WHERE run_id = ?1 AND node_id = ?2")?;
            let result = stmt.query_row(params![run_id_str, node_id_str], |row| {
                let data: String = row.get(0)?;
                Ok(data)
            });
            match result {
                Ok(data) => {
                    let checkpoint: StepCheckpoint = serde_json::from_str(&data)?;
                    Ok(Some(checkpoint))
                }
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(error) => Err(Box::new(error) as Box<dyn std::error::Error + Send + Sync>),
            }
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
        let conn = self.conn.clone();
        let run_id_str = run_id.to_string();
        Box::pin(async move {
            let conn = conn.lock();
            conn.execute(
                "DELETE FROM step_checkpoints WHERE run_id = ?1",
                params![run_id_str],
            )?;
            conn.execute(
                "DELETE FROM workflow_checkpoints WHERE run_id = ?1",
                params![run_id_str],
            )?;
            Ok(())
        })
    }
}
