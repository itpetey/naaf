use futures::future::LocalBoxFuture;
use naaf_llm::{Tool, ToolSpec};
use serde_json::{Value, json};

use crate::client::QdrantClient;
use crate::embedder::Embedder;
use crate::error::QdrantError;

/// LLM tool that exposes vector search over the configured Qdrant collection.
pub struct QdrantSearchTool<R> {
    client: QdrantClient,
    embedder: Box<dyn Embedder>,
    default_top_k: usize,
    default_min_score: f32,
    _marker: std::marker::PhantomData<R>,
}

impl<R> QdrantSearchTool<R> {
    /// Creates a search tool with default retrieval parameters.
    pub fn new(
        client: QdrantClient,
        embedder: Box<dyn Embedder>,
        default_top_k: usize,
        default_min_score: f32,
    ) -> Self {
        Self {
            client,
            embedder,
            default_top_k,
            default_min_score,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<R: 'static> Tool for QdrantSearchTool<R> {
    type Runtime = R;
    type Error = QdrantError;

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "knowledge_search".to_string(),
            description: "Search the knowledge base for relevant information. \
                Returns matching entries with their content, similarity scores, \
                and metadata."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Natural language search query"
                    },
                    "top_k": {
                        "type": "integer",
                        "description": "Number of results to return"
                    },
                    "min_score": {
                        "type": "number",
                        "description": "Minimum similarity score (0.0 to 1.0)"
                    },
                    "repo": {
                        "type": "string",
                        "description": "Repository name to filter by"
                    }
                },
                "required": ["query"]
            }),
        }
    }

    fn call<'a>(
        &'a self,
        _runtime: &'a Self::Runtime,
        arguments: Value,
    ) -> LocalBoxFuture<'a, Result<Value, Self::Error>> {
        let default_top_k = self.default_top_k;
        let default_min_score = self.default_min_score;
        Box::pin(async move {
            let query = arguments
                .get("query")
                .and_then(Value::as_str)
                .ok_or_else(|| QdrantError::InvalidPayload("missing 'query' field".to_string()))?
                .to_string();
            let top_k = arguments
                .get("top_k")
                .and_then(Value::as_u64)
                .unwrap_or(default_top_k as u64) as usize;
            let min_score = arguments
                .get("min_score")
                .and_then(Value::as_f64)
                .unwrap_or(default_min_score as f64) as f32;

            let vectors = self.embedder.embed(vec![query.clone()]).await?;
            let query_vector = vectors.into_iter().next().ok_or(QdrantError::NoResults)?;
            let repo = arguments.get("repo").and_then(Value::as_str);
            let results = self
                .client
                .search(query_vector, top_k, min_score, repo)
                .await?;

            let results_json: Vec<Value> = results
                .iter()
                .map(|r| {
                    json!({
                        "id": r.id.to_string(),
                        "score": r.score,
                        "title": r.payload.title,
                        "content": r.payload.content,
                        "entity_type": format!("{:?}", r.payload.entity_type),
                        "tags": r.payload.tags,
                    })
                })
                .collect();

            Ok(json!({
                "results": results_json,
                "query": query,
                "count": results_json.len(),
            }))
        })
    }
}
