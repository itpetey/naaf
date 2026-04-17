# Dynamic Workflows

Dynamic workflows construct their topology at runtime using `Workflow`, `StepNode`, and `GraphPatch`.

## Core Types

| Type | Purpose |
|---|---|
| `StepNode` | A node in the workflow graph |
| `NodeSpec` | Defines a step's requirements |
| `EdgeSpec` | Defines connections between nodes |
| `GraphPatch` | Adds nodes/edges at runtime |
| `Workflow` | Executes the directed acyclic graph |

## Creating Nodes

```rust
use naaf_core::{StepNode, NodeSpec, NodeInput};

let node = StepNode::new(
    Step::builder(MyTask)
        .validate(MyCheck)
        .build(),
    |input: &NodeInput| input.seed_as::<MyInput>(),
);
```

Each node has:
- A step to execute
- A closure that extracts/seeds input from node input

## Spawning Dynamic Nodes

Nodes can spawn new nodes at runtime:

```rust
let root = StepNode::new(
    Step::builder(RootStep).build(),
    |input: &NodeInput| input.seed_as::<RootInput>(),
).spawn_with(|context, output| {
    // Analyze output and decide what nodes to spawn
    match output.needs_subtasks() {
        true => GraphPatch::new()
            .with_node(child_node_a)
            .with_node(child_node_b)
            .with_edge(EdgeSpec::new(context.node_id, child_node_a_id)),
        false => GraphPatch::new(),
    }
});
```

## Building the Workflow

```rust
use naaf_core::{Workflow, GraphPatch};

let workflow = Workflow::new()
    .with_patch(GraphPatch::new()
        .with_node(root_spec)
        .with_node(node_a)
        .with_node(node_b)
        .with_edge(EdgeSpec::new(root_id, node_a_id))
    )?;

let report = workflow.run(&runtime).await?;
```

## Scheduling

The runtime schedules nodes when their dependencies complete:

- Nodes with no dependencies run immediately
- Nodes with satisfied dependencies run in parallel
- Downstream patches are applied when parent nodes complete

## Node Input

Each node receives a `NodeInput`:

```rust
fn extract_input(input: &NodeInput) -> MyType {
    // Get from seed
    input.seed_as::<MyType>()
    
    // Or from previous node output
    input.get_output::<PreviousOutput>(node_id)
}
```

## Example: Dynamic Task Generation

```rust
use naaf_core::{StepNode, Workflow, GraphPatch, NodeInput, EdgeSpec};

let plan_node = StepNode::new(
    Step::builder(PlanTasks).build(),
    |input: &NodeInput| input.seed_as::<ProjectBrief>(),
).spawn_with(|context, plan| {
    let mut patch = GraphPatch::new();

    for (i, task) in plan.tasks.iter().enumerate() {
        let task_node = StepNode::new(
            Step::builder(ExecuteTask)
                .validate(TaskCheck)
                .build(),
            |input: &NodeInput| input.seed_as::<TaskInput>(),
        );
        patch = patch
            .with_node(task_node)
            .with_edge(EdgeSpec::new(context.node_id, task_node_id));
    }

    patch
};

let workflow = Workflow::new()
    .with_patch(GraphPatch::new().with_node(plan_node))?;

let report = workflow.run(&runtime).await?;
```

## Error Handling

The workflow report contains results from all nodes:

```rust
let report = workflow.run(&runtime).await?;

for (node_id, result) in report.node_results() {
    match result {
        Ok(output) => println!("Node {}: {:?}", node_id, output),
        Err(e) => eprintln!("Node {} failed: {:?}", node_id, e),
    }
}
```

## See Also

- [Sequential Composition](sequential.md) — Linear chaining
- [Parallel Composition](parallel.md) — Concurrent execution