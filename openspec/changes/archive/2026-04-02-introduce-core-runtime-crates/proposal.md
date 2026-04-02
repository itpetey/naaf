## Why

The new workflow runtime requires dedicated crates with clear responsibilities. The current codebase has runtime concerns mixed with application logic. We need to introduce new core crates: `workflow-core`, `workflow-schema`, `workflow-llm`, and `workflow-builtins` to provide the foundation for the new architecture.

## What Changes

- Add 4 new crates to the workspace: `workflow-core`, `workflow-schema`, `workflow-llm`, `workflow-builtins`
- Create basic crate scaffolding with `Cargo.toml` and `lib.rs`
- Wire crates into workspace dependencies
- Add initial module structure per REFACTOR_PLAN.md skeleton

## Capabilities

### New Capabilities
- `workflow-core`: Core runtime crate with builder, executor, graph, steps, route, join, budget, events, errors
- `workflow-schema`: State envelope, artifacts, contracts, meta, lineage, validation
- `workflow-llm`: Model client, prompt rendering, structured output, repair, usage tracking
- `workflow-builtins`: Reusable step implementations for classification, normalization, validation

### Modified Capabilities
- (none yet - this is foundational)

## Impact

- New crate directories added to repository root
- Workspace `Cargo.toml` updated with new member crates
- No changes to existing legacy code
