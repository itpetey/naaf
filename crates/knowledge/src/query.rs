use naaf_qdrant::{QdrantAgent, SearchResult};

use crate::error::KnowledgeError;

/// High-level result structure for knowledge queries.
pub struct QueryResult {
    /// Synthesised answer text.
    pub answer: String,
    /// Source entries supporting the answer.
    pub sources: Vec<SearchResult>,
    /// Whether the answer was written back into the knowledge base.
    pub re_ingested: bool,
}

/// Executes a vector search against the knowledge collection.
pub async fn query_knowledge<R>(
    qdrant: &QdrantAgent<R>,
    query: &str,
    top_k: usize,
    min_score: f32,
    repo: Option<&str>,
) -> Result<Vec<SearchResult>, KnowledgeError> {
    qdrant
        .search(query, top_k, min_score, repo)
        .await
        .map_err(KnowledgeError::Qdrant)
}
