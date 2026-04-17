use thiserror::Error;

#[derive(Error, Debug)]
pub enum QdrantError {
    #[error("Qdrant client error: {0}")]
    Client(String),

    #[error("Embedding error: {0}")]
    Embedding(String),

    #[error("Chunking error: {0}")]
    Chunking(String),

    #[error("Collection not found: {0}")]
    CollectionNotFound(String),

    #[error("Point not found: {0}")]
    PointNotFound(String),

    #[error("Invalid payload: {0}")]
    InvalidPayload(String),

    #[error("Search returned no results")]
    NoResults,

    #[error("PDF extraction error: {0}")]
    PdfExtraction(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, QdrantError>;
