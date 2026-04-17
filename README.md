# naaf

**Not Another Agent Framework** — a strongly typed Rust library for building asynchronous LLM workflows out of retrying steps.

naaf provides composable, type-safe building blocks for workflows that plan, validate, repair, and rerun. The core loop is: run a task, validate the output, and if validation fails, repair the input and try again. Steps compose sequentially, in parallel, or as dynamic graphs — all with full tracing support.

You can find user documentation here: [itpetey.github.io/naaf/](https://itpetey.github.io/naaf/).

## Status

naaf is early-stage. APIs may change between releases.

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

## Quick Start

### The core loop: Task → Check → Repair

Every workflow starts with a **Task** that produces output, optionally validated by **Check**s, recovered by a **RepairPlanner**, and tracked through it all:

```rust
use naaf_core::{Check, RetryPolicy, Step, Task};

// Define your domain types
struct GeneratePatch;
struct CargoTest;
struct RepairPatch;

// Implement Task, Check, and RepairPlanner for your domain...

// Build a step with validation and automatic repair
let step = Step::builder(GeneratePatch)
    .materialise(ApplyPatch)        // transform output before checking
    .validate(CargoTest)            // check the materialised result
    .repair_with(RepairPatch)       // plan a new input on failure
    .retry_policy(RetryPolicy::new(3)) // allow up to 3 attempts
    .build();

// Run the step
let result = step.run(&runtime, prompt).await?;
```

When `CargoTest` returns findings, `RepairPatch` receives the full attempt history and produces a new input. The step retries until checks pass or the budget is exhausted.

### Composing steps

Steps compose into workflows using combinators:

```rust
// Sequential: left output feeds into right
let pipeline = step_a.then(step_b);

// Parallel fan-out: same input, both run concurrently
let both = step_a.join(step_b);

// Parallel with separate inputs
let pair = step_a.zip(step_b);

// Fan-in: reconcile parallel outputs with a task
let merged = both.reconcile_task(MergeResults);
```

### Inspecting results

Use `run_traced` to get the output alongside a full attempt report:

```rust
let traced = step.run_traced(&runtime, input).await?;
println!("Output: {:?}", traced.output());
println!("Attempts: {}", traced.report().attempt_count());
for attempt in traced.report().attempts() {
    println!("  accepted={}, findings={:?}", attempt.accepted(), attempt.findings);
}
```

## Core Concepts

### Traits

naaf is built around four core traits, all driven by a shared `Runtime`:

| Trait | Purpose | Key Method |
|---|---|---|
| `Task` | Produces an artefact from input | `run(&self, runtime, input)` |
| `Check` | Validates a subject, returns findings | `check(&self, runtime, subject)` |
| `Materialiser` | Transforms output (often with side effects) | `materialise(&self, runtime, input)` |
| `RepairPlanner` | Produces the next input from failed attempts | `repair(&self, runtime, attempts)` |

Each trait is async, takes a runtime reference for capabilities, and returns a strongly typed result. No stringly-typed dictionaries — your Rust types flow through the entire pipeline.

### Step

A `Step` owns one task plus its validation pipeline and optional repair loop. The builder pattern enforces type safety:

- `Step::builder(task)` — starts building
- `.validate(check)` — adds a check
- `.materialise(materialiser)` — transforms between checks
- `.repair_with(planner)` — enables retry on validation failure
- `.retry_policy(policy)` — configures max attempts (default: 1)
- `.build()` — produces a runnable step

Checks run in the order they are added. Materialisers convert the subject type between checks. An empty findings list means the step is accepted.

### Workflow

For runtime-determined topologies, `Workflow` builds a DAG of `StepNode`s:

```rust
use naaf_core::{StepNode, Workflow, NodeSpec, EdgeSpec, GraphPatch};

// Create typed step nodes
let plan_node = StepNode::new(
    Step::builder(PlanProject).with_findings::<()>().build(),
    |input: &NodeInput| input.seed_as::<ProjectBrief>(),
);

// Nodes can spawn downstream work via spawn_with()
let root = StepNode::new(
    Step::builder(root_step).with_findings::<()>().build(),
    |input: &NodeInput| input.seed_as::<Input>(),
).spawn_with(|_context, output| {
    // Return a GraphPatch with new nodes and edges
    GraphPatch::new()
        .with_node(node_a)
        .with_node(node_b)
        .with_edge(EdgeSpec::new(root_id, node_a_id))
});

let workflow = Workflow::new()
    .with_patch(GraphPatch::new().with_node(root_spec))?;

let report = workflow.run(&runtime).await?;
```

The scheduler executes nodes concurrently when dependencies are satisfied and applies downstream patches returned by completed nodes.

### Observability

All core traits have extension methods for structured `tracing` logs:

```rust
use naaf_core::TaskExt;

let observed_task = my_task.observed();           // auto-named
let observed_task = my_task.observed_as("name");  // custom name
```

Equivalent extensions exist for `Check`, `Materialiser`, and `RepairPlanner`. Steps also emit their own structured traces during execution.

## Crates

| Crate | Description |
|---|---|
| `naaf-core` | Core traits (`Task`, `Check`, `Materialiser`, `RepairPlanner`), `Step` builder, `Workflow` runtime, observability |
| `naaf-llm` | LLM-backed adapters (`LlmTask`, `LlmCheck`, `LlmMaterialiser`, `LlmRepairPlanner`), `LlmAgent`, executor with tool calling, OpenAI client |
| `naaf-process` | Shell-command adapters (`ProcessTask`, `ProcessCheck`, `ProcessMaterialiser`, `ProcessRepairPlanner`), `ProcessAgent` |
| `naaf-qdrant` | Qdrant vector database integration (`QdrantAgent`, search, upsert, similarity check), chunkers, embedding adapters |
| `naaf-knowledge` | Knowledge orchestration with Qdrant — ingest, query, and lint operations built on `naaf-core` |
| `naaf-tui` | Terminal UI for observing workflows, human-in-the-loop prompting |
| `naaf-persistence-fs` | Filesystem-based checkpoint persistence for workflow resumption |
| `naaf-persistence-sqlite` | SQLite-based checkpoint persistence for workflow resumption |
| `naaf-cli` | CLI binary (`naaf kb`) for ingesting and querying the knowledge base |

## LLM Integration

The `naaf-llm` crate provides a shared `LlmAgent` that can be projected into any core trait:

```rust
use naaf_llm::{LlmAgent, Executor, Message, CompletionRequest, ExecutionOutcome};

let agent = LlmAgent::new(client);

// Project into any role
let task = agent.task(
    |_, input: String| Ok(CompletionRequest::new("model", vec![Message::user(input)])),
    |outcome: ExecutionOutcome| Ok(outcome.final_message().content.clone().unwrap_or_default()),
);

let check = agent.check(
    |_, subject: String| Ok(CompletionRequest::new("model", vec![Message::user(subject)])),
    |outcome: ExecutionOutcome| serde_json::from_str(&outcome.final_message().content.as_deref().unwrap_or("[]")),
);
```

A single `LlmAgent` shares its executor across all projections, so one client configuration serves the entire workflow.

Tool calling is supported through `Tool`, `ToolRegistry`, and `ToolSpec`. The executor handles the full model → tool call → tool result → model loop automatically.

Dynamic graph spawning from LLM tool calls is available via `SpawnTool` and `resolve_spawn`.

## Process Integration

The `naaf-process` crate wraps shell commands into core traits:

```rust
use naaf_process::{ProcessAgent, ProcessCommand, ProcessOutput};

let agent = ProcessAgent::new();

let task = agent.task(
    |_, script: String| Ok(ProcessCommand::shell(script)),
    |output: ProcessOutput| String::from_utf8(output.stdout),
);
```

The same agent can produce `ProcessCheck`, `ProcessMaterialiser`, and `ProcessRepairPlanner` adapters.

## Examples

Each example is a standalone binary in the `examples/` directory. Run them with:

```bash
cargo run -p <example-name>
```

| Example | Description |
|---|---|
| [step-retry](examples/step-retry/) | Task-check-repair loop with a planning task that validates and retries |
| [materialiser](examples/materialiser/) | Materialising task output into a different type before validation |
| [join-reconcile](examples/join-reconcile/) | Parallel fan-out with `.join()` and fan-in with `.reconcile_task()` |
| [composed-workflow](examples/composed-workflow/) | Sequenced retry + parallel composition |
| [dynamic-workflow](examples/dynamic-workflow/) | Runtime graph construction with `Workflow`, `StepNode`, and `GraphPatch` |
| [process-task](examples/process-task/) | Shell-command integration via `naaf-process` |
| [build-test](examples/build-test/) | Generate → materialise → validate → repair loop at the heart of naaf |
| [knowledge-tool](examples/knowledge-tool/) | Knowledge base integration with Qdrant |
| [tui-demo](examples/tui-demo/) | Terminal UI for observing workflows |

## Error Handling

Steps produce a `StepError<F, E>`:

- `StepError::System { stage, error }` — infrastructure failure (task, validation, or repair stage)
- `StepError::Rejected(StepReport<F>)` — exhausted retries with the full attempt history

The `StepReport<F>` contains every `AttemptReport<F>`, giving you access to all findings from every attempt.

## Building & Testing

```bash
# Build
cargo build

# Run all tests
cargo test

# Format and lint
cargo fmt --all
cargo clippy -- -D warnings
```

Requires Rust edition 2024.

## Licence

[MPL-2.0](LICENCE)
