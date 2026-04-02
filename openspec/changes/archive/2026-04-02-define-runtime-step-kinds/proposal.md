## Why

The workflow runtime needs explicit step abstractions. Currently, transformers handle both transformation and routing implicitly. We need first-class traits for Transformer, Router, Reducer, and Validator that make the step type explicit in the workflow definition.

## What Changes

- Define `Transformer` trait: consumes state, produces new state
- Define `Router` trait: consumes state, decides next edge(s)
- Define `Reducer` trait: consumes multiple states, produces merged state
- Define `Validator` trait: checks state invariants
- Define `StepError` and `ValidationError` types
- Define boxed wrappers for dynamic dispatch

## Capabilities

### New Capabilities
- `transformer-trait`: Step trait for state transformation
- `router-trait`: Step trait for routing decisions
- `reducer-trait`: Step trait for fan-in/join operations
- `validator-trait`: Step trait for state validation

### Modified Capabilities
- (none yet)

## Impact

- New traits in `workflow-core` crate
- Required for workflow builder (Phase 4) and executor (Phase 5)
