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
//! - `step_checkpoints` — stores per-step checkpoints by run ID and no

use std::sync::Arc;

use naaf_core::{NodeId, StepCheckpoint, WorkflowCheckpoint, WorkflowRunId};
use naaf_knowledge::{KnowledgeGroup, KnowledgeGroupStore, KnowledgeGroupStoreFuture};
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

/// SQLite-backed persistence for knowledge-group metadata.
pub struct SqliteKnowledgeGroupStore {
    conn: Arc<Mutex<Connection>>,
}

#[derive(Debug, Error)]
pub enum SqliteKnowledgeGroupStoreError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("serialisation error: {0}")]
    Serialisation(#[from] serde_json::Error),
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

impl SqliteKnowledgeGroupStore {
    /// Creates a knowledge-group store over an existing SQLite connection.
    pub fn new(conn: Connection) -> Result<Self, SqliteKnowledgeGroupStoreError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS knowledge_groups (
                collection TEXT PRIMARY KEY,
                data TEXT NOT NULL
            );",
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Opens a knowledge-group store backed by the SQLite database at `path`.
    pub fn open(path: &str) -> Result<Self, SqliteKnowledgeGroupStoreError> {
        let conn = Connection::open(path)?;
        Self::new(conn)
    }

    /// Opens an in-memory knowledge-group store.
    pub fn open_in_memory() -> Result<Self, SqliteKnowledgeGroupStoreError> {
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

impl KnowledgeGroupStore for SqliteKnowledgeGroupStore {
    fn upsert_group(&self, group: &KnowledgeGroup) -> KnowledgeGroupStoreFuture<()> {
        let conn = self.conn.clone();
        let group = group.clone();
        Box::pin(async move {
            let conn = conn.lock();
            let mut stmt =
                conn.prepare("SELECT data FROM knowledge_groups WHERE collection = ?1")?;
            let existing = match stmt.query_row(params![group.collection.as_str()], |row| {
                let data: String = row.get(0)?;
                Ok(data)
            }) {
                Ok(data) => Some(serde_json::from_str::<KnowledgeGroup>(&data)?),
                Err(rusqlite::Error::QueryReturnedNoRows) => None,
                Err(error) => {
                    return Err(Box::new(error) as Box<dyn std::error::Error + Send + Sync>);
                }
            };
            drop(stmt);

            let stored_group = group.prepare_for_upsert(existing.as_ref());
            let data = serde_json::to_string(&stored_group)?;

            conn.execute(
                "INSERT OR REPLACE INTO knowledge_groups (collection, data) VALUES (?1, ?2)",
                params![stored_group.collection, data],
            )?;
            Ok(())
        })
    }

    fn load_group(&self, collection: &str) -> KnowledgeGroupStoreFuture<Option<KnowledgeGroup>> {
        let conn = self.conn.clone();
        let collection = collection.to_string();
        Box::pin(async move {
            let conn = conn.lock();
            let mut stmt =
                conn.prepare("SELECT data FROM knowledge_groups WHERE collection = ?1")?;
            let result = stmt.query_row(params![collection], |row| {
                let data: String = row.get(0)?;
                Ok(data)
            });
            match result {
                Ok(data) => Ok(Some(serde_json::from_str(&data)?)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(error) => Err(Box::new(error) as Box<dyn std::error::Error + Send + Sync>),
            }
        })
    }

    fn list_groups(&self) -> KnowledgeGroupStoreFuture<Vec<KnowledgeGroup>> {
        let conn = self.conn.clone();
        Box::pin(async move {
            let conn = conn.lock();
            let mut stmt = conn.prepare("SELECT data FROM knowledge_groups ORDER BY collection")?;
            let rows = stmt.query_map([], |row| {
                let data: String = row.get(0)?;
                Ok(data)
            })?;

            let mut groups = Vec::new();
            for row in rows {
                groups.push(serde_json::from_str::<KnowledgeGroup>(&row?)?);
            }
            Ok(groups)
        })
    }

    fn delete_group(&self, collection: &str) -> KnowledgeGroupStoreFuture<()> {
        let conn = self.conn.clone();
        let collection = collection.to_string();
        Box::pin(async move {
            let conn = conn.lock();
            conn.execute(
                "DELETE FROM knowledge_groups WHERE collection = ?1",
                params![collection],
            )?;
            Ok(())
        })
    }
}
