# Knowledge Base

`naaf-knowledge` integrates Qdrant-backed retrieval with `naaf-llm` without making `naaf-llm` itself knowledge-aware.

The recommended pattern is:

1. Persist and select `KnowledgeGroup`s in application code.
2. Build a `KnowledgeLlmSession` from those groups.
3. Reuse the generated system prompt and tool registry across requests.

## Recommended Pattern

```rust
use naaf_knowledge::{KnowledgeGroup, KnowledgeLlmSessionBuilder};
use naaf_llm::{Executor, ExecutorConfig, OpenAiClient, OpenAiConfig};

let qdrant = naaf_qdrant::QdrantClient::from_url(
    "http://localhost:6333",
    Option::<String>::None,
)?
.with_collection("docs");
let embedder = naaf_qdrant::OpenAiEmbedder::new(std::env::var("OPENAI_API_KEY")?);

let knowledge = KnowledgeLlmSessionBuilder::new(Box::new(embedder))
    .with_system_prompt("You are a helpful assistant for the workspace.")
    .with_group(
        KnowledgeGroup::new("docs", "Documentation", "Product and API documentation"),
        qdrant,
    )
    .with_search_defaults(5, 0.7)
    .with_lint_tool(true)
    .build()?;

let llm_client = OpenAiClient::new(OpenAiConfig::new(std::env::var("OPENAI_API_KEY")?));
let executor = Executor::with_tools(llm_client, knowledge.tools().clone())
    .with_config(ExecutorConfig::new(5));

let request = knowledge.request_with_user_message("gpt-4o", "How do steps retry?");
let outcome = executor.execute(&(), request).await?;
println!("{}", outcome.final_message().content.as_deref().unwrap_or(""));
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Why This Shape

- `naaf-llm` stays generic and only owns execution and tool calling.
- `naaf-knowledge` owns knowledge groups, prompt augmentation, and knowledge-specific tools.
- The application chooses which groups are exposed for each request.

## Main Types

- `KnowledgeGroup`: metadata describing one exposed collection.
- `KnowledgeLlmSessionBuilder`: high-level helper that creates the prompt and tools together.
- `KnowledgeLlmSession`: reusable session object for request building.
- `KnowledgeSearchTool`: lower-level retrieval tool for custom wiring.
- `KnowledgeLintTool`: lower-level lint tool for custom wiring.

## Prompt Augmentation

If you need more control than `KnowledgeLlmSessionBuilder` provides, `naaf-knowledge` also exposes prompt primitives:

```rust
use naaf_knowledge::{
    KnowledgePromptConfig, augment_system_prompt, format_knowledge_prompt_block,
};

let prompt_config = KnowledgePromptConfig::default();
let system_prompt = augment_system_prompt(
    "You are a helpful assistant.",
    &groups,
    &prompt_config,
);

let block = format_knowledge_prompt_block(&groups, &prompt_config);
```

## When To Drop Down A Level

Use `KnowledgeSearchTool` and `KnowledgeLintTool` directly when:

1. You already have your own prompt-building pipeline.
2. You want custom `CompletionRequest` assembly.
3. You want to expose only search or only lint.

## See Also

- [LLM Integration](llm.md)
- [Examples](../examples.md)
