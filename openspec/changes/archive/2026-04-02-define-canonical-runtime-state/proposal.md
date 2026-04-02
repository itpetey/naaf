## Why

The new workflow runtime needs a canonical state representation that all steps operate on. Currently, the prototype uses different runtime structs for different workflow states. We need a single `StateEnvelope` type that serves as the universal runtime payload.

## What Changes

- Define `StateEnvelope` with id, run_id, kind, artifacts, meta, lineage
- Define support types: `StateId`, `RunId`, `StateKind`, `ArtifactMap`, `ArtifactKey`, `ArtifactValue`, `StateMeta`, `Lineage`
- Implement typed artifact storage with `ArtifactValue` enum
- Define initial `StateKind` enum: Proposed, Normalized, Scoped, Planned, Accepted, Ambiguous, Escalated, Terminal
- Add serialization support for all state types

## Capabilities

### New Capabilities
- `canonical-state-envelope`: Unified state representation for all workflow steps
- `typed-artifacts`: Structured artifact storage with typed values
- `state-lineage`: Tracking state transitions for debugging

### Modified Capabilities
- (none yet)

## Impact

- New types in `workflow-schema` crate
- All subsequent phases depend on these types
