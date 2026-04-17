# Task

The `Task` trait produces an artefact from input. It's the starting point of every workflow.

## Definition

```rust
#[async_trait::async_trait]
pub trait Task: Send + Sync {
    type Input;
    type Output;
    type Error;

    async fn run(&self, runtime: &Runtime, input: &Self::Input) -> Result<Self::Output, Self::Error>;
}
```

## Parameters

- `Input` — The type of data this task consumes
- `Output` — The type of data this task produces
- `Error` — The error type this task may return

## Example

```rust
use naaf_core::{Runtime, Task};

struct GenerateCode;

#[async_trait::async_trait]
impl Task for GenerateCode {
    type Input = String;
    type Output = String;
    type Error = std::io::Error;

    async fn run(&self, _: &Runtime, input: &String) -> Result<String, std::io::Error> {
        Ok(format!("fn solution() {{ // {}\n    unimplemented!()\n}}", input))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = Runtime::new();
    let task = GenerateCode;

    let output = task.run(&runtime, &"add two numbers".to_string()).await?;
    println!("{}", output);

    Ok(())
}
```

## Using Runtime Capabilities

The runtime provides access to shared capabilities:

```rust
use naaf_core::{Runtime, Task};
use naaf_llm::LlmClient;

struct LlmTask {
    prompt: String,
}

#[async_trait::async_trait]
impl Task for LlmTask {
    type Input = String;
    type Output = String;
    type Error = std::io::Error;

    async fn run(&self, runtime: &Runtime, input: &String) -> Result<String, std::io::Error> {
        let llm = runtime.llm().ok_or_else(|| std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "LLM not configured"
        ))?;

        let full_prompt = format!("{}: {}", self.prompt, input);
        let response = llm.complete(&full_prompt).await?;
        Ok(response)
    }
}
```

## Task Combinators

Tasks can be combined using the `TaskExt` extension trait:

```rust
use naaf_core::TaskExt;

// Wrap a task with observability
let observed = my_task.observed();

// Wrap with a custom name
let named = my_task.observed_as("generate_code");
```

## See Also

- [Check](../check.md) — Validates task output
- [Materialiser](../materialiser.md) — Transforms task output
- [Step](../step.md) — Combines Task with Check and RepairPlanner