# Check

The `Check` trait validates a subject and returns findings. An empty findings list means the check passed.

## Definition

```rust
#[async_trait::async_trait]
pub trait Check: Send + Sync {
    type Subject;
    type Findings;
    type Error;

    async fn check(
        &self,
        runtime: &Runtime,
        subject: &Self::Subject,
    ) -> Result<Vec<Self::Findings>, Self::Error>;
}
```

## Parameters

- `Subject` — The type of data this check validates
- `Findings` — The type of findings this check may return
- `Error` — The error type this check may return

## Key Concept: Empty Findings

An empty findings list (`vec![]`) means the subject passed validation. Any non-empty list indicates failure.

## Example

```rust
use naaf_core::{Check, Runtime};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ValidationError {
    message: String,
    line: Option<u32>,
}

struct SyntaxCheck;

#[async_trait::async_trait]
impl Check for SyntaxCheck {
    type Subject = String;
    type Findings = ValidationError;
    type Error = std::io::Error;

    async fn check(
        &self,
        _: &Runtime,
        subject: &String,
    ) -> Result<Vec<ValidationError>, std::io::Error> {
        let mut findings = vec![];

        for (i, line) in subject.lines().enumerate() {
            if line.contains("unimplemented!()") && !line.starts_with("//") {
                findings.push(ValidationError {
                    message: "Unimplemented code found".to_string(),
                    line: Some(i as u32 + 1),
                });
            }
        }

        Ok(findings)
    }
}
```

## Chaining Checks

Multiple checks form a validation pipeline:

```rust
let step = Step::builder(GenerateCode)
    .validate(SyntaxCheck)
    .validate(SecurityCheck)
    .validate(PerformanceCheck)
    .build();
```

Checks run in the order they are added. The step only passes if all checks return empty findings.

## Using Findings in Repair

Findings feed directly into the repair process:

```rust
use naaf_core::RepairPlanner;

struct RepairCode;

#[async_trait::async_trait]
impl RepairPlanner for RepairCode {
    type Input = String;
    type Output = String;
    type Findings = ValidationError;

    async fn repair(
        &self,
        _: &Runtime,
        attempts: &[naaf_core::Attempt<ValidationError, String>],
    ) -> Result<String, Self::Error> {
        let findings = attempts.last()
            .and_then(|a| a.findings())
            .unwrap_or(&vec![]);

        let mut instructions = String::new();
        for finding in findings {
            instructions.push_str(&format!("// TODO: {}\n", finding.message));
        }

        Ok(format!("// Repair based on {} findings\n{}",
            findings.len(), instructions))
    }
}
```

## See Also

- [Task](task.md) — Produces the subject checked
- [Materialiser](materialiser.md) — Transforms output before checking
- [Recipe](step.md) — Combines Task with Check and RepairPlanner