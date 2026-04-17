# Step

A `Step` owns one task plus its validation pipeline and optional repair loop. The builder pattern enforces type safety at compile time.

## Creating a Step

```rust
use naaf_core::{Check, RetryPolicy, Step, Task};

let step = Step::builder(GenerateCode)
    .validate(SyntaxCheck)
    .validate(SecurityCheck)
    .repair_with(RepairCode)
    .retry_policy(RetryPolicy::new(3))
    .build();
```

## Builder Methods

| Method | Description | Returns |
|---|---|---|
| `Step::builder(task)` | Start building with a task | `StepBuilder` |
| `.validate(check)` | Add a check to the pipeline | `&mut StepBuilder` |
| `.materialise(materialiser)` | Add a materialiser | `&mut StepBuilder` |
| `.repair_with(planner)` | Enable retry on failure | `&mut StepBuilder` |
| `.retry_policy(policy)` | Configure max attempts | `&mut StepBuilder` |
| `.build()` | Produce a runnable step | `Step` |

## Type Safety

The builder enforces that types are compatible:

```rust
// This won't compile - type mismatch:
// Task output (String) doesn't match Check subject (u32)
Step::builder(GenerateString)
    .validate(ValidateNumber)  // Error!
    .build();
```

## Retry Policy

```rust
use naaf_core::RetryPolicy;

// Default: 1 attempt (no retry)
let policy = RetryPolicy::default();

// Custom: up to N attempts
let policy = RetryPolicy::new(3);

// Exponential backoff
let policy = RetryPolicy::new(3).with_backoff(
    tokio::time::Duration::from_secs(1),
    tokio::time::Duration::from_secs(10),
);
```

## Running a Step

### Basic run

```rust
let output = step.run(&runtime, &input).await?;
```

### Run with tracing

```rust
use naaf_core::TracedOutput;

let traced = step.run_traced(&runtime, &input).await?;
println!("Output: {:?}", traced.output());
println!("Attempts: {}", traced.report().attempt_count());

for attempt in traced.report().attempts() {
    println!("  accepted={}, findings={:?}", attempt.accepted(), attempt.findings());
}
```

## Error Handling

Steps produce a `StepError<F, E>`:

```rust
use naaf_core::StepError;

match result {
    Ok(output) => println!("Success: {:?}", output),
    Err(StepError::System { stage, error }) => {
        eprintln!("Infrastructure failure at {}: {:?}", stage, error);
    }
    Err(StepError::Rejected(report)) => {
        eprintln!("Exhausted retries after {} attempts", report.attempt_count());
        for attempt in report.attempts() {
            eprintln!("  - findings: {:?}", attempt.findings());
        }
    }
}
```

## Step Outputs

A `Step` can be used as a `Task`, `Check`, or `Materialiser` in other contexts:

```rust
use naaf_core::{Check, Task};

// Using a step as a Task in another step
let composed = Step::builder(step)  // step implements Task
    .validate(FinalCheck)
    .build();
```

## Finding Types

Steps track findings throughout execution. Access them via the report:

```rust
let traced = step.run_traced(&runtime, &input).await?;
let report = traced.report();

println!("Total attempts: {}", report.attempt_count());
println!("Final state: {:?}", report.final_state());
```

## See Also

- [Task](task.md) — The task this step wraps
- [Check](check.md) — The validation checks
- [Materialiser](materialiser.md) — Type transformations
- [RepairPlanner](repair-planner.md) — Retry logic