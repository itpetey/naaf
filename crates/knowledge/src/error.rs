use thiserror::Error;

#[derive(Error, Debug)]
pub enum KnowledgeError {
    #[error("Ingest error: {0}")]
    Ingest(String),

    #[error("Query error: {0}")]
    Query(String),

    #[error("Lint error: {0}")]
    Lint(String),

    #[error("Source detection error: {0}")]
    SourceDetection(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Qdrant error: {0}")]
    Qdrant(#[from] naaf_qdrant::QdrantError),
}

pub type Result<T> = std::result::Result<T, KnowledgeError>;
