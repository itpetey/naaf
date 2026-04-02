## Why

The current prototype mixes workflow stages (state kinds) with readiness/execution statuses. This violates design principle #8. We need to separate semantic state kind, execution status, and workflow outcome into distinct concepts.

## What Changes

- Define separate `ExecutionStatus` enum: Pending, Running, Succeeded, Failed
- Define separate `WorkflowOutcome` enum: Completed, NeedHumanClarification, Rejected, Escalated, Aborted
- Ensure StateKind remains semantic-only
- Remove mixed semantics from any legacy phase-like abstractions

## Capabilities

### New Capabilities
- `execution-status`: Separate concept for runtime execution state
- `workflow-outcome`: Separate concept for final outcome

### Modified Capabilities
- `canonical-state-envelope`: Updated to use separated concepts

## Impact

- Updates to `workflow-schema` crate
- Clean semantic model
