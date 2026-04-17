# Knowledge Base

The `naaf-knowledge` crate provides knowledge orchestration with Qdrant vector database.

## Setup

```toml
[dependencies]
naaf-knowledge = "0.1.0"
naaf-qdrant = "0.1.0"
```

## Architecture

```
User Input → KnowledgeIngest → Qdrant
              ↓
Query Request → KnowledgeQuery → Qdrant → LLM Context
              ↓
Lint Request → KnowledgeLint → Qdrant → LLM Feedback
```

## QdrantClient

```rust
use naaf_qdrant::QdrantClient;

let client = QdrantClient::new(
    "http://localhost:6333",
    Some("api-key"),
);
```

## Knowledge Operations

### Ingest

```rust
use naaf_knowledge::{IngestRequest, IngestResponse};

let request = IngestRequest::new()
    .with_documents(docs)
    .with_chunking_config(ChunkingConfig::default()
        .chunk_size(1024)
        .overlap(128))
    .with_embeddings(embeddings_config);

let response = knowledge.ingest(&runtime, request).await?;
```

### Query

```rust
use naaf_knowledge::{QueryRequest, QueryResponse};

let request = QueryRequest::new()
    .with_query("how do I build a step?")
    .with_limit(5)
    .with_threshold(0.7);

let response = knowledge.query(&runtime, request).await?;

for result in response.results() {
    println!("Score: {:.2}, Content: {}", result.score(), result.content());
}
```

### Lint

```rust
use naaf_knowledge::{LintRequest, LintResponse};

let request = LintRequest::new()
    .with_code(code)
    .with_context(context_docs);

let response = knowledge.lint(&runtime, request).await?;
```

## Chunking Strategies

### Fixed Size

```rust
use naaf_qdrant::ChunkingConfig;

ChunkingConfig::default()
    .chunk_size(1024)
    .overlap(128)
```

### Semantic

Split by paragraphs, sections, or code blocks:

```rust
ChunkingConfig::semantic()
    .separators(&["\n\n", "\n", " "])
    .min_chunk_size(100)
    .max_chunk_size(2048)
```

## Embeddings

```rust
use naaf_qdrant::{EmbeddingAdapter, EmbeddingRequest};

let adapter = naaf_qdrant::OpenAIEmbeddings::new(client.clone(), "text-embedding-ada-002");

let request = EmbeddingRequest::new()
    .with_texts(texts);

let embeddings = adapter.embed(&runtime, request).await?;
```

## Collections

```rust
use naaf_qdrant::{client, CreateCollection};

client.create_collection("my-collection").await?;

let collection = client.collection("my-collection");
```

## CLI

The `naaf-cli` crate provides a CLI for knowledge base operations:

```bash
# Ingest documents
naaf kb ingest --path /path/to/docs

# Query the knowledge base
naaf kb query "how do I build a workflow?"

# Start the API server
naaf kb serve --port 8080
```

## Example: Full Workflow

```rust
use naaf_knowledge::{KnowledgeOrchestrator, IngestRequest, QueryRequest};
use naaf_core::Step;

let orchestrator = KnowledgeOrchestrator::new(
    qdrant_client,
    llm_client,
);

// Ingest documentation
tokio::spawn(async move {
    orchestrator.ingest(&runtime, IngestRequest::new()
        .with_documents(load_docs("docs/").await?)
    ).await.unwrap();
});

// Query with context
let result = orchestrator.query(&runtime, QueryRequest::new()
    .with_query(user_query)
    .with_limit(3)
).await?;
```

## See Also

- [Qdrant Integration](llm.md) — Vector database
- [Examples](examples.md) — Knowledge base examples