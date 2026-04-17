# LLM Integration

The `naaf-llm` crate provides LLM-backed adapters, tool calling, and multiple provider support.

## Setup

```toml
[dependencies]
naaf-llm = "0.1.0"
```

With OpenAI support:

```toml
naaf-llm = { version = "0.1.0", features = ["openai"] }
```

## LlmClient

```rust
use naaf_llm::LlmClient;

let client = LlmClient::builder()
    .api_key(std::env::var("OPENAI_API_KEY")?)
    .model("gpt-4")
    .build();
```

## LlmAgent

The `LlmAgent` provides a shared executor that can be projected into any core trait:

```rust
use naaf_llm::{LlmAgent, Message, CompletionRequest, ExecutionOutcome};

let agent = LlmAgent::new(client);

// Project into Task
let task = agent.task(
    |_, input: String| Ok(CompletionRequest::new("gpt-4", vec![Message::user(input)])),
    |outcome: ExecutionOutcome| Ok(outcome.final_message().content.clone().unwrap_or_default()),
);

// Project into Check
let check = agent.check(
    |_, subject: String| Ok(CompletionRequest::new("gpt-4", vec![Message::user(subject)])),
    |outcome: ExecutionOutcome| serde_json::from_str(&outcome.final_message().content.as_deref().unwrap_or("[]")),
);

// Project into RepairPlanner
let repair = agent.repair(
    |_, attempts: Vec<Attempt<Findings, Input>| { ... },
    |outcome: ExecutionOutcome| Ok(outcome.final_message().content.clone().unwrap_or_default()),
);
```

## Tool Calling

Define tools for the LLM to use:

```rust
use naaf_llm::{Tool, ToolRegistry, ToolSpec};

struct SearchTool;

impl Tool for SearchTool {
    type Input = String;
    type Output = Vec<SearchResult>;

    fn spec(&self) -> ToolSpec {
        ToolSpec::new("search", "Search the knowledge base")
            .add_parameter("query", "string", "The search query")
    }

    async fn execute(&self, runtime: &Runtime, input: &String) -> Result<Vec<SearchResult>, Self::Error> {
        // Execute the tool
        Ok(search(&runtime, input).await?)
    }
}

let registry = ToolRegistry::new()
    .with_tool(SearchTool);
```

## Dynamic Spawning

Spawn new nodes from tool calls:

```rust
use naaf_llm::{SpawnTool, SpawnResult, resolve_spawn};

let spawn_tool = SpawnTool::new(registry);

let outcome = llm.execute_with_tools(&request, &[spawn_tool]).await?;

if let Some(SpawnResult { spawn, .. }) = outcome.tool_calls().first() {
    let patch = resolve_spawn(spawn, node_context).await?;
    workflow.apply_patch(patch)?;
}
```

## Built-in Adapters

| Adapter | Description |
|---|---|
| `LlmTask` | Project LLM as a Task |
| `LlmCheck` | Project LLM as a Check |
| `LlmMaterialiser` | Project LLM as a Materialiser |
| `LlmRepairPlanner` | Project LLM as a RepairPlanner |

## Providers

### OpenAI

```rust
use naaf_llm::OpenAIClient;

let client = OpenAIClient::new(api_key, "gpt-4");
```

### Anthropic

```rust
use naaf_llm::AnthropicClient;

let client = AnthropicClient::new(api_key, "claude-3-opus-20240229");
```

### Custom Provider

Implement the `LlmProvider` trait for other providers:

```rust
impl LlmProvider for MyProvider {
    async fn complete(&self, request: &CompletionRequest) -> Result<CompletionResponse, Self::Error>;
    async fn complete_with_tools(&self, request: &CompletionRequest, tools: &[Box<dyn Tool>]) -> Result<CompletionResponse, Self::Error>;
}
```