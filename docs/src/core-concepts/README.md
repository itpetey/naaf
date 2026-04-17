# Core Concepts

naaf is built around four core traits, all driven by a shared `Runtime`. These traits form the building blocks of every workflow.

## The Four Traits

| Trait | Purpose | Key Method |
|---|---|---|
| `Task` | Produces an artefact from input | `run(&self, runtime, input)` |
| `Check` | Validates a subject, returns findings | `check(&self, runtime, subject)` |
| `Materialiser` | Transforms output (often with side effects) | `materialise(&self, runtime, input)` |
| `RepairPlanner` | Produces the next input from failed attempts | `repair(&self, runtime, attempts)` |

Each trait is async, takes a runtime reference for capabilities, and returns a strongly typed result. No stringly-typed dictionaries — your Rust types flow through the entire pipeline.

## Relationship Between Traits

```
Input → Task → Output → Materialiser → Subject → Check → Findings
                    ↑                              ↓
                    ←←←←←← RepairPlanner ←←←←←←←←←←←←
```

The core loop works as follows:

1. A `Task` produces an `Output`
2. A `Materialiser` optionally transforms the output into a `Subject`
3. A `Check` validates the subject and returns `Findings`
4. If findings are non-empty, a `RepairPlanner` can produce a new input for retry

## Runtime

The `Runtime` provides shared capabilities to all traits:

```rust
use naaf_core::Runtime;

let runtime = Runtime::new();
```

You can extend the runtime with capabilities:

```rust
use naaf_llm::LlmClient;
use naaf_qdrant::QdrantClient;

// Extend with LLM and vector database capabilities
let runtime = Runtime::new()
    .with_llm(LlmClient::new())
    .with_vector_db(QdrantClient::new(url, api_key));
```

## Guide to This Section

- [Task](task.md) — Produces output from input
- [Check](check.md) — Validates output, returns findings
- [Materialiser](materialiser.md) — Transforms output between stages
- [RepairPlanner](repair-planner.md) — Plans retry input from failures
- [Step](step.md) — Combines Task, Check, Materialiser, and RepairPlanner