# RepairPlanner

The `RepairPlanner` trait produces a new input from failed attempts. It's the intelligence behind the retry loop.

## Definition

```rust
#[async_trait::async_trait]
pub trait RepairPlanner: Send + Sync {
    type Input;
    type Output;
    type Findings;

    async fn repair(
        &self,
        runtime: &Runtime,
        attempts: &[Attempt<Self::Findings, Self::Input>],
    ) -> Result<Self::Output, Self::Error>;
}
```

## Parameters

- `Input` — The type of input that started the attempt
- `Output` — The new input to retry with
- `Findings` — The type of findings from failed checks

## Accessing Attempt History

The `repair` method receives all previous attempts, giving full context for repair decisions:

```rust
use naaf_core::{Attempt, RepairPlanner, Runtime};

struct LlmRepairPlanner {
    system_prompt: String,
}

#[async_trait::async_trait]
impl RepairPlanner for LlmRepairPlanner {
    type Input = String;
    type Output = String;
    type Findings = ValidationError;

    async fn repair(
        &self,
        runtime: &Runtime,
        attempts: &[Attempt<ValidationError, String>],
    ) -> Result<String, Self::Error> {
        let last_attempt = attempts.last().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::Other, "No attempts")
        })?;

        let history = attempts.iter().enumerate().map(|(i, a)| {
            format!("Attempt {}:\nInput: {}\nFindings: {:?}",
                i + 1, a.input(), a.findings())
        }).join("\n---\n");

        let prompt = format!(
            "{}\n\nPrevious attempts:\n{}\n\nProduce a new input:",
            self.system_prompt, history
        );

        let llm = runtime.llm().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "LLM not configured")
        })?;

        let response = llm.complete(&prompt).await?;
        Ok(response)
    }
}
```

## Attempt Structure

Each `Attempt<F, I>` contains:

| Method | Description |
|---|---|
| `input()` | The original input for this attempt |
| `output()` | The task's output |
| `subject()` | The materialised subject (if any) |
| `findings()` | The findings from checks |
| `accepted()` | Whether the step passed validation |

## Repair Strategies

### Simple retry with modified input

```rust
struct IncrementRetry;

#[async_trait::async_trait]
impl RepairPlanner for IncrementRetry {
    type Input = u32;
    type Output = u32;
    type Findings = ();

    async fn repair(
        &self,
        _: &Runtime,
        attempts: &[Attempt<(), u32>],
    ) -> Result<u32, Self::Error> {
        Ok(attempts.len() as u32 + 1)
    }
}
```

### LLM-based repair

Uses an LLM to analyze failures and generate better input:

```rust
struct SmartRepair {
    model: String,
    system_prompt: String,
}

#[async_trait::async_trait]
impl RepairPlanner for SmartRepair {
    type Input = String;
    type Output = String;
    type Findings = ValidationError;

    async fn repair(
        &self,
        runtime: &Runtime,
        attempts: &[Attempt<ValidationError, String>],
    ) -> Result<String, Self::Error> {
        // Analyze findings and ask LLM for new input
        let llm = runtime.llm().expect("LLM configured");
        let response = llm.complete(&build_repair_prompt(self.system_prompt, attempts)).await?;
        Ok(response)
    }
}
```

## Enabling Retry

A step only retries when a `RepairPlanner` is attached:

```rust
let step = Step::builder(GenerateCode)
    .validate(SyntaxCheck)
    .repair_with(LlmRepairPlanner { system_prompt: "Fix the code".to_string() })
    .retry_policy(RetryPolicy::new(3))  // Allow up to 3 attempts
    .build();
```

Without `.repair_with()`, the step does not retry — it returns immediately after the first failure.

## See Also

- [Step](step.md) — Combines Task, Check, Materialiser, and RepairPlanner
- [RetryPolicy](step.md#retry-policy) — Configures retry behavior
- [LLM Integration](../integrations/llm.md) — Use LLMs for repair planning