# naaf - Not Another Agent Framework

naaf is a strongly typed Rust library for building asynchronous LLM workflows out of retrying steps. It provides composable, type-safe building blocks for workflows that plan, validate, repair, and rerun.

## Core Philosophy

Every workflow follows the core loop: **run a task, validate the output, and if validation fails, repair the input and try again**. Steps compose sequentially, in parallel, or as dynamic graphs — all with full tracing support.

## Status

naaf is early-stage. APIs may change between releases.

## Features

- **Type-safe workflows** — Your Rust types flow through the entire pipeline. No stringly-typed dictionaries.
- **Composable steps** — Sequential, parallel, and dynamic graph composition.
- **Built-in retry** — Task-check-repair loop with configurable retry policies.
- **Observability** — Full tracing support for debugging and monitoring.
- **Multiple backends** — LLM integration (OpenAI, Anthropic), shell commands, and vector databases.

## Crates

| Crate | Description |
|---|---|
| `naaf-core` | Core traits, Step builder, Workflow runtime |
| `naaf-llm` | LLM-backed adapters, tool calling |
| `naaf-process` | Shell-command adapters |
| `naaf-qdrant` | Qdrant vector database integration |
| `naaf-knowledge` | Knowledge orchestration |
| `naaf-tui` | Terminal UI for workflow observation |
| `naaf-persistence-fs` | Filesystem checkpoint persistence |
| `naaf-persistence-sqlite` | SQLite checkpoint persistence |
| `naaf-cli` | CLI for knowledge base operations |

## Quick Example

```rust
use naaf_core::{Check, RetryPolicy, Step, Task};

// Define your domain types
struct GeneratePatch;
struct CargoTest;
struct RepairPatch;

// Build a step with validation and automatic repair
let step = Step::builder(GeneratePatch)
    .validate(CargoTest)
    .repair_with(RepairPatch)
    .retry_policy(RetryPolicy::new(3))
    .build();

// Run the step
let result = step.run(&runtime, prompt).await?;
```

## Next Steps

- [Getting Started](getting-started.md) — Install and run your first workflow
- [Core Concepts](core-concepts/README.md) — Understand the building blocks
- [Examples](examples.md) — Run standalone examples