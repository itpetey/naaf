## 1. Define graph structures

- [x] 1.1 Create `workflow-core/src/graph.rs`
- [x] 1.2 Define `GraphNode` enum with Transformer, Router, Reducer, Validator variants
- [x] 1.3 Define `GraphEdge` with source, target, edge type
- [x] 1.4 Define `CompiledWorkflow` struct with nodes, edges, entry point

## 2. Implement WorkflowBuilder

- [x] 2.1 Create `workflow-core/src/builder.rs`
- [x] 2.2 Implement `WorkflowBuilder::new(name)`
- [x] 2.3 Implement `step()`, `route()`, `branch()`, `path()`, `join()`, `terminal()` methods
- [x] 2.4 Implement `compile()` method

## 3. Implement validation

- [x] 3.1 Add unique step ID validation
- [x] 3.2 Add reference existence validation
- [x] 3.3 Add terminal path validation
- [x] 3.4 Add join reducer validation
- [x] 3.5 Add acyclicity validation

## 4. Verify build

- [x] 4.1 Run `cargo build -p workflow-core`
- [x] 4.2 Fix any compilation errors