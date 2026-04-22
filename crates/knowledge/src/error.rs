use thiserror::Error;

/// Convenience result type used by this crate.
pub type Result<T> = std::result::Result<T, KnowledgeError>;

/// Errors returned by knowledge ingestion, query, and lint operations.
#[derive(Error, Debug)]
pub enum KnowledgeError {
    /// An ingestion pipeline step failed.
    #[error("Ingest error: {0}")]
    Ingest(String),

    /// A query-time operation failed.
    #[error("Query error: {0}")]
    Query(String),

    /// Linting failed.
    #[error("Lint error: {0}")]
    Lint(String),

    /// Source classification or conversion failed.
    #[error("Source detection error: {0}")]
    SourceDetection(String),

    /// An underlying file system operation failed.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialisation or deserialisation failed.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// The underlying Qdrant integration returned an error.
    #[error("Qdrant error: {0}")]
    Qdrant(#[from] naaf_qdrant::QdrantError),
}
