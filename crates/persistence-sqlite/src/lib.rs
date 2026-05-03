//! SQLite-based persistence for `naaf`.
//!
//! `naaf-persistence-sqlite` provides a knowledge-group store implementation
//! backed by SQLite.

use std::sync::Arc;

use naaf_knowledge::{KnowledgeGroup, KnowledgeGroupStore, KnowledgeGroupStoreFuture};
use parking_lot::Mutex;
use rusqlite::{Connection, params};
use thiserror::Error;

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
