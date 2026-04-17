# Observability

naaf provides structured tracing for debugging and monitoring workflows. All core traits have extension methods for observability.

## TaskExt

Wrap any task with observability:

```rust
use naaf_core::TaskExt;

let observed = my_task.observed();           // Auto-named
let observed = my_task.observed_as("name");  // Custom name
```

## CheckExt

Wrap checks:

```rust
use naaf_core::CheckExt;

let observed_check = my_check.observed();
let observed_check = my_check.observed_as("validate_output");
```

## Running with Tracing

Use `run_traced()` to get detailed execution reports:

```rust
use naaf_core::TracedOutput;

let traced = step.run_traced(&runtime, &input).await?;

println!("Output: {:?}", traced.output());
println!("Attempts: {}", traced.report().attempt_count());

for attempt in traced.report().attempts() {
    println!("  accepted={}, findings={:?}", 
        attempt.accepted(), 
        attempt.findings()
    );
}
```

## AttemptReport

Each attempt contains:

| Method | Description |
|---|---|
| `input()` | Original input |
| `output()` | Task output |
| `subject()` | Materialised subject |
| `findings()` | Validation findings |
| `accepted()` | Whether check passed |

## TracedOutput

```rust
struct TracedOutput<F, O> {
    output: O,
    report: StepReport<F>,
}
```

Methods:
- `output()` — Get the final output
- `report()` — Get the execution report

## StepReport

```rust
struct StepReport<F> {
    attempts: Vec<AttemptReport<F>>,
}
```

Methods:
- `attempt_count()` — Total attempts made
- `attempts()` — Iterator over attempts
- `final_state()` — Final acceptance state
- `findings()` — All findings from all attempts

## Structured Logging

naaf uses the `tracing` crate:

```rust
use tracing::{info, error, warn};

info!(step = "generate", "running step");
warn!(findings = ?findings, "validation failed, retrying");
error!(attempt = 3, "retry exhausted");
```

## Configuration

Enable tracing in your runtime:

```rust
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::*;

tracing_subscriber::registry()
    .with(fmt::layer())
    .init();
```

## See Also

- [Examples](examples.md) — See observability in action