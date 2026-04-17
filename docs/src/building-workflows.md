# Building Workflows

Workflows compose individual steps into larger patterns. naaf provides three composition models:

1. **Sequential** — Output from one step feeds into the next
2. **Parallel** — Steps run concurrently, then reconcile
3. **Dynamic** — Runtime-determined topology with `Workflow`

## Sequential Workflows

The simplest pattern: chain steps where output flows to input.

```rust
use naaf_core::Step;

// Sequential: A → B → C
let workflow = step_a
    .then(step_b)
    .then(step_c);
```

The output type of each step must match the input type of the next.

## Parallel Workflows

Run multiple steps concurrently:

```rust
// Fan-out: same input, both run
let both = step_a.join(step_b);

// Fan-out: different inputs
let pair = step_a.zip(step_b);

// Fan-in: reconcile parallel outputs
let merged = both.reconcile_task(MergeResults);
```

## Dynamic Workflows

For runtime-determined topologies, use `Workflow` with nodes:

```rust
use naaf_core::{StepNode, Workflow, NodeSpec, EdgeSpec, GraphPatch};

// Create typed step nodes
let plan_node = StepNode::new(
    Step::builder(PlanProject).with_findings::<()>().build(),
    |input: &NodeInput| input.seed_as::<ProjectBrief>(),
);

// Nodes can spawn downstream work
let root = StepNode::new(
    Step::builder(root_step).with_findings::<()>().build(),
    |input: &NodeInput| input.seed_as::<Input>(),
).spawn_with(|_context, output| {
    GraphPatch::new()
        .with_node(node_a)
        .with_node(node_b)
        .with_edge(EdgeSpec::new(root_id, node_a_id))
});

let workflow = Workflow::new()
    .with_patch(GraphPatch::new().with_node(root_spec))?;

let report = workflow.run(&runtime).await?;
```

## Guide to Composition

- [Sequential Composition](composing-steps/sequential.md) — Chain steps linearly
- [Parallel Composition](composing-steps/parallel.md) — Fan-out and fan-in
- [Dynamic Workflows](composing-steps/dynamic.md) — Runtime graph construction