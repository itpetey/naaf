use naaf_qdrant::{QdrantAgent, SearchResult};

use crate::error::KnowledgeError;

pub struct QueryResult {
    pub answer: String,
    pub sources: Vec<SearchResult>,
    pub re_ingested: bool,
}

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
