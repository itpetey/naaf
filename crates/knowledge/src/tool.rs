use std::cmp::Ordering;

use futures::future::LocalBoxFuture;
use naaf_llm::{Tool, ToolSpec};
use naaf_qdrant::{Embedder, QdrantClient};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{KnowledgeError, KnowledgeGroup};

#[derive(Clone)]
struct KnowledgeTarget {
    group: KnowledgeGroup,
    client: QdrantClient,
}

#[derive(Debug, Deserialize)]
struct KnowledgeSearchParams {
    query: String,
    #[serde(default)]
    collection: Option<String>,
    #[serde(default)]
    top_k: Option<usize>,
    #[serde(default)]
    min_score: Option<f32>,
    #[serde(default)]
    repo: Option<String>,
}

#[derive(Debug, Deserialize)]
struct KnowledgeLintParams {
    #[serde(default)]
    collection: Option<String>,
}

/// LLM tool that searches one or more configured knowledge groups.
pub struct KnowledgeSearchTool<R> {
    targets: Vec<KnowledgeTarget>,
    embedder: Box<dyn Embedder>,
    default_top_k: usize,
    default_min_score: f32,
    repo: Option<String>,
    _marker: std::marker::PhantomData<R>,
}

/// LLM tool that lints one or more configured knowledge groups.
pub struct KnowledgeLintTool<R> {
    targets: Vec<KnowledgeTarget>,
    _marker: std::marker::PhantomData<R>,
}

impl<R> KnowledgeSearchTool<R> {
    /// Creates a search tool with fixed retrieval defaults.
    pub fn new(embedder: Box<dyn Embedder>, default_top_k: usize, default_min_score: f32) -> Self {
        Self {
            targets: Vec::new(),
            embedder,
            default_top_k,
            default_min_score,
            repo: None,
            _marker: std::marker::PhantomData,
        }
    }

    /// Adds one searchable knowledge group together with its Qdrant client.
    pub fn with_group(mut self, group: KnowledgeGroup, client: QdrantClient) -> Self {
        self.targets.push(KnowledgeTarget { group, client });
        self
    }

    /// Restricts searches to a single repository label by default.
    pub fn with_repo(mut self, repo: impl Into<String>) -> Self {
        self.repo = Some(repo.into());
        self
    }
}

impl<R> KnowledgeLintTool<R> {
    /// Creates an empty lint tool.
    pub fn new() -> Self {
        Self {
            targets: Vec::new(),
            _marker: std::marker::PhantomData,
        }
    }

    /// Adds one lintable knowledge group together with its Qdrant client.
    pub fn with_group(mut self, group: KnowledgeGroup, client: QdrantClient) -> Self {
        self.targets.push(KnowledgeTarget { group, client });
        self
    }
}

impl<R> Default for KnowledgeLintTool<R> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R: 'static> Tool for KnowledgeSearchTool<R> {
    type Runtime = R;
    type Error = KnowledgeError;

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "knowledge_search".to_string(),
            description: "Search the configured knowledge groups for relevant information. Returns matching entries together with the collection they came from.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Natural-language search query"
                    },
                    "collection": collection_schema(&self.targets),
                    "top_k": {
                        "type": "integer",
                        "description": "Maximum number of merged results to return"
                    },
                    "min_score": {
                        "type": "number",
                        "description": "Minimum similarity score to keep"
                    },
                    "repo": {
                        "type": "string",
                        "description": "Repository label to filter by"
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
        Box::pin(async move {
            let params: KnowledgeSearchParams = serde_json::from_value(arguments)
                .map_err(|error| KnowledgeError::Query(format!("invalid arguments: {error}")))?;
            let targets = select_targets(&self.targets, params.collection.as_deref(), |message| {
                KnowledgeError::Query(message)
            })?;
            let top_k = params.top_k.unwrap_or(self.default_top_k);
            let min_score = params.min_score.unwrap_or(self.default_min_score);
            let repo = params.repo.as_deref().or(self.repo.as_deref());
            let query = params.query;

            let vectors = self
                .embedder
                .embed(vec![query.clone()])
                .await
                .map_err(|error| KnowledgeError::Query(error.to_string()))?;
            let query_vector = vectors
                .into_iter()
                .next()
                .ok_or_else(|| KnowledgeError::Query("embedding failed".to_string()))?;

            let searched_collections = targets
                .iter()
                .map(|target| target.group.collection.clone())
                .collect::<Vec<_>>();
            let mut merged_results = Vec::new();

            for target in targets {
                let results = target
                    .client
                    .search(query_vector.clone(), top_k, min_score, repo)
                    .await
                    .map_err(KnowledgeError::Qdrant)?;
                merged_results.extend(results.into_iter().map(|result| {
                    json!({
                        "collection": target.group.collection.clone(),
                        "score": result.score,
                        "title": result.payload.title,
                        "content": result.payload.content,
                        "entity_type": format!("{:?}", result.payload.entity_type),
                        "tags": result.payload.tags,
                    })
                }));
            }

            merged_results.sort_by(|left, right| {
                let left_score = left
                    .get("score")
                    .and_then(Value::as_f64)
                    .unwrap_or_default();
                let right_score = right
                    .get("score")
                    .and_then(Value::as_f64)
                    .unwrap_or_default();
                right_score
                    .partial_cmp(&left_score)
                    .unwrap_or(Ordering::Equal)
            });
            merged_results.truncate(top_k);

            Ok(json!({
                "query": query,
                "searched_collections": searched_collections,
                "count": merged_results.len(),
                "results": merged_results,
            }))
        })
    }
}

impl<R: 'static> Tool for KnowledgeLintTool<R> {
    type Runtime = R;
    type Error = KnowledgeError;

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "knowledge_lint".to_string(),
            description: "Lint the configured knowledge groups for graph and metadata issues."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "collection": collection_schema(&self.targets)
                }
            }),
        }
    }

    fn call<'a>(
        &'a self,
        _runtime: &'a Self::Runtime,
        arguments: Value,
    ) -> LocalBoxFuture<'a, Result<Value, Self::Error>> {
        Box::pin(async move {
            let params: KnowledgeLintParams = serde_json::from_value(arguments)
                .map_err(|error| KnowledgeError::Lint(format!("invalid arguments: {error}")))?;
            let targets = select_targets(&self.targets, params.collection.as_deref(), |message| {
                KnowledgeError::Lint(message)
            })?;
            let mut collections = Vec::new();
            let mut total_issues = 0usize;

            for target in targets {
                let report = crate::lint::lint_collection(&target.client).await?;
                total_issues += report.issues.len();
                collections.push(json!({
                    "collection": target.group.collection.clone(),
                    "name": target.group.name.clone(),
                    "entries_scanned": report.entries_scanned,
                    "issues_count": report.issues.len(),
                    "issues": report.issues.iter().map(|issue| json!({
                        "type": format!("{:?}", issue.issue_type),
                        "description": issue.description,
                        "entry_ids": issue.entry_ids,
                        "suggestion": issue.suggestion,
                    })).collect::<Vec<_>>(),
                }));
            }

            Ok(json!({
                "count": total_issues,
                "collections": collections,
            }))
        })
    }
}

fn collection_schema(targets: &[KnowledgeTarget]) -> Value {
    let mut collections = targets
        .iter()
        .map(|target| target.group.collection.clone())
        .collect::<Vec<_>>();
    collections.sort();
    collections.dedup();

    let mut schema = json!({
        "type": "string",
        "description": "Optional canonical collection id from the available knowledge groups"
    });

    if !collections.is_empty() {
        schema["enum"] = json!(collections);
    }

    schema
}

fn select_targets<'a>(
    targets: &'a [KnowledgeTarget],
    collection: Option<&str>,
    make_error: impl Fn(String) -> KnowledgeError,
) -> Result<Vec<&'a KnowledgeTarget>, KnowledgeError> {
    if targets.is_empty() {
        return Err(make_error("no knowledge groups configured".to_string()));
    }

    if let Some(collection) = collection {
        let selected = targets
            .iter()
            .filter(|target| target.group.collection == collection)
            .collect::<Vec<_>>();
        if selected.is_empty() {
            return Err(make_error(format!(
                "unknown knowledge collection: {collection}"
            )));
        }
        Ok(selected)
    } else {
        Ok(targets.iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use futures::future::LocalBoxFuture;
    use naaf_llm::Tool;
    use naaf_qdrant::{Embedder, QdrantClient, QdrantError};
    use serde_json::json;

    use super::{KnowledgeLintTool, KnowledgeSearchTool};
    use crate::KnowledgeGroup;

    struct StubEmbedder;

    impl Embedder for StubEmbedder {
        fn embed<'a>(
            &'a self,
            texts: Vec<String>,
        ) -> LocalBoxFuture<'a, Result<Vec<Vec<f32>>, QdrantError>> {
            Box::pin(async move { Ok(texts.into_iter().map(|_| vec![1.0, 2.0]).collect()) })
        }

        fn dimension(&self) -> usize {
            2
        }
    }

    #[test]
    fn search_tool_schema_advertises_collection_enum() {
        let client = QdrantClient::from_url("http://localhost:6333", Option::<String>::None)
            .expect("client should build")
            .with_collection("docs");
        let tool = KnowledgeSearchTool::<()>::new(Box::new(StubEmbedder), 5, 0.7).with_group(
            KnowledgeGroup::new("docs", "Documentation", "Docs collection"),
            client,
        );

        let schema = tool.spec().input_schema;

        assert_eq!(schema["properties"]["collection"]["enum"], json!(["docs"]));
    }

    #[test]
    fn lint_tool_schema_advertises_collection_enum() {
        let client = QdrantClient::from_url("http://localhost:6333", Option::<String>::None)
            .expect("client should build")
            .with_collection("docs");
        let tool = KnowledgeLintTool::<()>::new().with_group(
            KnowledgeGroup::new("docs", "Documentation", "Docs collection"),
            client,
        );

        let schema = tool.spec().input_schema;

        assert_eq!(schema["properties"]["collection"]["enum"], json!(["docs"]));
    }
}
