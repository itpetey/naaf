## Why

Workflows need to be declared declaratively and compiled before execution. Currently, there's no workflow definition model. We need a builder API that allows declarative workflow definition and compile-time validation.

## What Changes

- Create `WorkflowBuilder` API with methods: step, route, branch, path, join, terminal
- Define compiled graph representation with nodes and edges
- Implement compile-time validation: unique step IDs, all references exist, no disconnected nodes, one or more terminal paths, joins have reducers, graph is acyclic
- Define validation error types

## Capabilities

### New Capabilities
- `workflow-builder`: Declarative DSL for workflow definition
- `workflow-compilation`: Compile workflow to executable graph with validation
- `compiled-graph`: Graph representation of compiled workflow

### Modified Capabilities
- (none yet)

## Impact

- New builder API in `workflow-core` crate
- Foundation for Phase 5 (executor)
