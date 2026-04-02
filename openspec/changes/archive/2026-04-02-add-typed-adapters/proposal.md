## Why

The runtime uses StateEnvelope, but individual steps may want typed local inputs/outputs. We need typed adapters that allow steps to operate on domain views while the runtime stays envelope-based.

## What Changes

- Define `TryFromState` trait for typed state extraction
- Define `IntoState` trait for typed state conversion
- Implement typed transformer adapter wrappers
- Ensure adapter errors are clean and actionable

## Capabilities

### New Capabilities
- `typed-state-adapters`: Traits for typed state access

### Modified Capabilities
- (none yet)

## Impact

- New traits in `workflow-schema` crate
- Ergonomic step authoring
