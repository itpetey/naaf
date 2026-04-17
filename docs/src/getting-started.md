# Getting Started

This guide walks you through installing naaf and running your first workflow.

## Installation

Add the crates you need to your `Cargo.toml`:

```toml
[dependencies]
naaf-core = "0.1.0"    # Core traits and Step builder
naaf-llm = "0.1.0"     # LLM-backed Task, Check, Materialiser, RepairPlanner
naaf-process = "0.1.0"  # Shell-command-backed adapters
```

The `naaf-llm` crate has an optional `openai` feature for the built-in OpenAI client:

```toml
naaf-llm = { version = "0.1.0", features = ["openai"] }
```

## Your First Workflow

This example creates a step that generates a code patch, validates it with cargo test, and retries on failure.

### Dependencies

```toml
[dependencies]
naaf-core = "0.1.0"
naaf-process = "0.1.0"
tokio = { version = "1", features = ["full"] }
```

### Code

```rust
use naaf_core::{Check, RetryPolicy, Step, Task};
use naaf_process::ProcessAgent;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Patch(String);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PatchFindings {
    failed: Vec<String>,
}

struct GeneratePatch;

#[async_trait::async_trait]
impl Task for GeneratePatch {
    type Input = String;
    type Output = Patch;
    type Error = std::io::Error;

    async fn run(&self, _: &naaf_core::Runtime, input: &String) -> Result<Patch, Self::Error> {
        Ok(Patch(format!("// Generated patch for: {}\nfn solution() {{ }}\n", input)))
    }
}

struct ValidatePatch;

#[async_trait::async_trait]
impl Check for ValidatePatch {
    type Subject = Patch;
    type Findings = PatchFindings;
    type Error = std::io::Error;

    async fn check(
        &self,
        _: &naaf_core::Runtime,
        subject: &Patch,
    ) -> Result<Vec<Self::Findings>, Self::Error> {
        let output = Command::new("sh")
            .arg("-c")
            .arg(format!("echo '{}' | cargo check --message-format=json 2>&1 || true", subject.0))
            .stdout(Stdio::piped())
            .output()
            .await?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        if output_str.contains("error") {
            Ok(vec![PatchFindings {
                failed: vec![output_str.to_string()],
            }])
        } else {
            Ok(vec![])
        }
    }
}

struct RepairPatch;

#[async_trait::async_trait]
impl naaf_core::RepairPlanner for RepairPatch {
    type Input = String;
    type Output = String;
    type Findings = PatchFindings;

    async fn repair(
        &self,
        _: &naaf_core::Runtime,
        attempts: &[naaf_core::Attempt<PatchFindings, Patch>],
    ) -> Result<String, Self::Error> {
        let last_input = attempts.last().map(|a| a.input()).unwrap_or("");
        let findings = attempts.last().map(|a| a.findings()).unwrap_or(&vec![]);

        Ok(format!("retry with: {} failures: {:?}", last_input, findings))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = naaf_core::Runtime::new();

    let step = Step::builder(GeneratePatch)
        .validate(ValidatePatch)
        .repair_with(RepairPatch)
        .retry_policy(RetryPolicy::new(3))
        .build();

    let result = step.run(&runtime, &"test input".to_string()).await;

    match result {
        Ok(output) => println!("Success: {:?}", output),
        Err(e) => println!("Failed: {:?}", e),
    }

    Ok(())
}
```

## Running Examples

Each example is a standalone binary in the `examples/` directory:

```bash
# Run the step-retry example
cargo run -p step-retry

# Run the materialiser example
cargo run -p materialiser

# Run the dynamic-workflow example
cargo run -p dynamic-workflow
```

## Next Steps

- [Core Concepts](core-concepts/README.md) — Understand the building blocks in detail
- [Building Workflows](building-workflows.md) — Compose steps into larger workflows
- [Integrations](integrations/README.md) — Connect to LLMs, shell commands, and databases