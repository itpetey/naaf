## Why

Workflow execution must be durable and traceable. Every step transition needs to emit an event for debugging and replay. Currently, there's no event system.

## What Changes

- Define event schema with types: run started, step entered, prompt rendered, provider called, provider responded, artifacts parsed, validator passed/failed, route selected, branch started, branch completed, join reduced, run terminated, run failed
- Define TraceSink trait for emitting events
- Implement filesystem-based event store
- Ensure replay/inspection is possible

## Capabilities

### New Capabilities
- `execution-events`: Event schema for all step transitions
- `trace-sink`: Abstraction for event emission
- `event-store`: Filesystem-based event persistence

### Modified Capabilities
- (none yet)

## Impact

- New event types in `workflow-core` crate
- Enables debugging and replay
