## Why

Workflows need to declare what they accept and guarantee to enable composition without structural coupling. Currently, there's no contract system.

## What Changes

- Define `WorkflowContract` struct with accepted_kinds, required_artifacts, guaranteed_artifacts, possible_output_kinds
- Define contract compatibility validation
- Implement composition helper
- Add adapter support for reshaping outputs

## Capabilities

### New Capabilities
- `workflow-contract`: Contract declaration for workflows
- `contract-compatibility`: Validation for contract matching
- `workflow-adapter`: Adapter for output reshaping

### Modified Capabilities
- (none yet)

## Impact

- New types in `workflow-schema` crate
- Enables workflow composition
