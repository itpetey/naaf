use thiserror::Error;

/// Convenience result type used by this crate.
pub type Result<T> = std::result::Result<T, QdrantError>;

/// Errors returned by Qdrant integration and content processing code.
#[derive(Error, Debug)]
pub enum QdrantError {
    /// The Qdrant client returned an error.
    #[error("Qdrant client error: {0}")]
    Client(String),

    /// Embedding generation failed.
    #[error("Embedding error: {0}")]
    Embedding(String),

    /// Source content could not be chunked.
    #[error("Chunking error: {0}")]
    Chunking(String),

    /// The configured collection does not exist.
    #[error("Collection not found: {0}")]
    CollectionNotFound(String),

    /// A referenced point could not be found.
    #[error("Point not found: {0}")]
    PointNotFound(String),

    /// Stored or generated payload data was invalid.
    #[error("Invalid payload: {0}")]
    InvalidPayload(String),

    /// A search completed without returning any usable result.
    #[error("Search returned no results")]
    NoResults,

    /// PDF text extraction failed.
    #[error("PDF extraction error: {0}")]
    PdfExtraction(String),

    /// An underlying file system operation failed.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialisation or deserialisation failed.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
