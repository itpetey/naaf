# Materialiser

The `Materialiser` trait transforms task output into a different type, often with side effects. It's positioned between the task and validation stages.

## Definition

```rust
#[async_trait::async_trait]
pub trait Materialiser: Send + Sync {
    type Input;
    type Output;
    type Error;

    async fn materialise(
        &self,
        runtime: &Runtime,
        input: &Self::Input,
    ) -> Result<Self::Output, Self::Error>;
}
```

## Parameters

- `Input` — The type this materialiser consumes (typically Task's output)
- `Output` — The type this materialiser produces (the Check's subject)
- `Error` — The error type this materialiser may return

## When to Use Materialiser

- **Type transformation** — Convert LLM output to executable code
- **File operations** — Write task output to disk before validation
- **API calls** — Transform output and submit to external services
- **Resource acquisition** — Allocate resources needed for validation

## Example: Write to File

```rust
use naaf_core::{Materialiser, Runtime};
use std::path::PathBuf;

struct WriteToFile {
    path: PathBuf,
}

#[async_trait::async_trait]
impl Materialiser for WriteToFile {
    type Input = String;
    type Output = PathBuf;
    type Error = std::io::Error;

    async fn materialise(
        &self,
        _: &Runtime,
        input: &String,
    ) -> Result<PathBuf, std::io::Error> {
        tokio::fs::write(&self.path, input).await?;
        Ok(self.path.clone())
    }
}
```

Usage in a step:

```rust
use naaf_core::Step;

let step = Step::builder(GenerateCode)
    .materialise(WriteToFile { path: PathBuf::from("/tmp/output.rs") })
    .validate(CheckCompiled)
    .build();
```

## Example: Transform to Executable Format

```rust
use naaf_core::{Materialiser, Runtime};

struct ParseToJson;

#[async_trait::async_trait]
impl Materialiser for ParseToJson {
    type Input = String;  // Raw LLM output
    type Output = serde_json::Value;  // Parsed JSON
    type Error = serde_json::Error;

    async fn materialise(
        &self,
        _: &Runtime,
        input: &String,
    ) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::from_str(input)
    }
}
```

The materialiser transforms raw string output into structured JSON that the check can validate.

## Multiple Materialisers

Chain materialisers when multiple transformations are needed:

```rust
let step = Step::builder(GenerateConfig)
    .materialise(ParseConfig)           // String → Value
    .materialise(ValidateSchema)       // Value → ValidatedValue
    .materialise(WriteToFile)          // ValidatedValue → PathBuf
    .validate(CheckFileExists)
    .build();
```

Each materialiser's output type must match the next materialiser's input type.

## See Also

- [Task](task.md) — Produces the input for materialisation
- [Check](check.md) — Validates the materialised output
- [Step](step.md) — Combines all stages