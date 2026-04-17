# Examples

Each example is a standalone binary in the `examples/` directory. Run them with `cargo run -p <example-name>`.

## Running Examples

```bash
# List all examples
ls examples/

# Run a specific example
cargo run -p step-retry

# Run with output visible
cargo run -p step-retry -- --nocapture
```

## Example Index

| Example | Description |
|---|---|
| [step-retry](https://github.com/itpetey/naaf/tree/main/examples/step-retry) | Task-check-repair loop with a planning task that validates and retries |
| [materialiser](https://github.com/itpetey/naaf/tree/main/examples/materialiser) | Materialising task output into a different type before validation |
| [join-reconcile](https://github.com/itpetey/naaf/tree/main/examples/join-reconcile) | Parallel fan-out with `.join()` and fan-in with `.reconcile_task()` |
| [composed-workflow](https://github.com/itpetey/naaf/tree/main/examples/composed-workflow) | Sequential retry + parallel composition |
| [dynamic-workflow](https://github.com/itpetey/naaf/tree/main/examples/dynamic-workflow) | Runtime graph construction with `Workflow`, `StepNode`, and `GraphPatch` |
| [process-task](https://github.com/itpetey/naaf/tree/main/examples/process-task) | Shell-command integration via `naaf-process` |
| [build-test](https://github.com/itpetey/naaf/tree/main/examples/build-test) | Generate → materialise → validate → repair loop at the heart of naaf |
| [knowledge-tool](https://github.com/itpetey/naaf/tree/main/examples/knowledge-tool) | Knowledge base integration |
| [tui-demo](https://github.com/itpetey/naaf/tree/main/examples/tui-demo) | Terminal UI demonstration |

## Example Descriptions

### step-retry

The core example: a task that generates output, validates it with a check, and retries with a repair planner on failure.

### materialiser

Shows how to transform output between task and check. A task produces raw text, a materialiser writes it to a file, and the check validates the file.

### join-reconcile

Parallel execution: use `.join()` to run two steps with the same input concurrently, then `.reconcile_task()` to merge their results.

### composed-workflow

Combines sequential and parallel composition: step A → (step B + step C) → step D.

### dynamic-workflow

Runtime-determined topology: nodes can spawn new nodes based on their output using `spawn_with()` and `GraphPatch`.

### process-task

Shell command integration: wraps `cargo check`, `cargo test`, and other CLI tools as tasks and checks.

### build-test

The quintessential workflow: generate code → write to file → compile → test → fix if needed.

### knowledge-tool

Qdrant integration: ingest documentation and query it with semantic search.

### tui-demo

Interactive terminal UI for observing workflow execution and providing human feedback.

## Adding New Examples

1. Create a new directory in `examples/`
2. Add a `Cargo.toml` with the crate name
3. Implement your example
4. Add it to `Cargo.toml` workspace members

```toml
# examples/my-example/Cargo.toml
[package]
name = "my-example"
version = "0.1.0"
edition = "2024"

[dependencies]
naaf-core = { path = "../crates/core" }
tokio = { version = "1", features = ["full"] }
```