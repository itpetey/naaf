use futures::future::LocalBoxFuture;
use naaf_llm::{Tool, ToolSpec};
use naaf_qdrant::{Embedder, QdrantClient};
use serde_json::{Value, json};

use crate::error::KnowledgeError;

pub struct KnowledgeTool<R> {
    client: QdrantClient,
    embedder: Box<dyn Embedder>,
    top_k: usize,
    min_score: f32,
    _marker: std::marker::PhantomData<R>,
}

impl<R> KnowledgeTool<R> {
    pub fn new(
        client: QdrantClient,
        embedder: Box<dyn Embedder>,
        top_k: usize,
        min_score: f32,
    ) -> Self {
        Self {
            client,
            embedder,
            top_k,
            min_score,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<R: 'static> Tool for KnowledgeTool<R> {
    type Runtime = R;
    type Error = KnowledgeError;

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "knowledge".to_string(),
            description: "Search and manage the knowledge base. \
                Supports query and lint operations."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["query", "lint"],
                        "description": "The operation to perform"
                    },
                    "query": {
                        "type": "string",
                        "description": "Search query (for query action)"
                    },
                    "top_k": {
                        "type": "integer",
                        "description": "Number of results (for query action)"
                    },
                    "repo": {
                        "type": "string",
                        "description": "Repository name to filter by (for query action)"
                    }
                },
                "required": ["action"]
            }),
        }
    }

    fn call<'a>(
        &'a self,
        _runtime: &'a Self::Runtime,
        arguments: Value,
    ) -> LocalBoxFuture<'a, Result<Value, Self::Error>> {
        let client = self.client.clone();
        let top_k = self.top_k;
        let min_score = self.min_score;
        Box::pin(async move {
            let action = arguments
                .get("action")
                .and_then(Value::as_str)
                .ok_or_else(|| KnowledgeError::Query("missing 'action' field".to_string()))?;

            match action {
                "query" => {
                    let query_text =
                        arguments
                            .get("query")
                            .and_then(Value::as_str)
                            .ok_or_else(|| {
                                KnowledgeError::Query("missing 'query' field".to_string())
                            })?;
                    let repo = arguments.get("repo").and_then(Value::as_str);
                    let vectors = self
                        .embedder
                        .embed(vec![query_text.to_string()])
                        .await
                        .map_err(|e| KnowledgeError::Query(e.to_string()))?;
                    let query_vector = vectors
                        .into_iter()
                        .next()
                        .ok_or_else(|| KnowledgeError::Query("embedding failed".to_string()))?;
                    let results = client
                        .search(query_vector, top_k, min_score, repo)
                        .await
                        .map_err(KnowledgeError::Qdrant)?;

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
                        "action": "query",
                        "results": results_json,
                        "count": results_json.len(),
                    }))
                }
                "lint" => {
                    let report = crate::lint::lint_collection(&client).await?;
                    Ok(json!({
                        "action": "lint",
                        "issues_count": report.issues.len(),
                        "entries_scanned": report.entries_scanned,
                        "issues": report.issues.iter().map(|i| json!({
                            "type": format!("{:?}", i.issue_type),
                            "description": i.description,
                            "suggestion": i.suggestion,
                        })).collect::<Vec<_>>(),
                    }))
                }
                _ => Err(KnowledgeError::Query(format!("unknown action: {action}"))),
            }
        })
    }
}
