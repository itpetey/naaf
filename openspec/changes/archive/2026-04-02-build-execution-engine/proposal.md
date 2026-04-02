## Why

The workflow runtime needs an execution engine to run compiled DAG workflows. Currently, there's no executor. We need to implement one that can invoke steps, handle routing, handle fan-out, wait for join completion, invoke reducers, emit events, and stop on terminal or fatal failure.

## What Changes

- Create `Executor` struct with execute method
- Define `ExecCtx` with run_id, budget, services, trace, cancel
- Implement single-state progression
- Implement route evaluation
- Implement branch spawning
- Implement join resolution
- Implement terminal handling
- Implement budget enforcement
- Add error propagation

## Capabilities

### New Capabilities
- `workflow-executor`: Runs compiled workflows
- `execution-context`: Runtime context for step execution
- `budget-enforcement`: Limits on steps, branches, tokens, time

### Modified Capabilities
- (none yet)

## Impact

- New executor in `workflow-core` crate
- Required for running workflows
