## Why

The orchestrator can now execute workflows end-to-end, but there's no user-facing interface to run and inspect executions. We need a CLI that allows users to supply prompts, execute the happy-path workflow, and inspect persisted artifacts and journal entries. This provides the first supervised operator experience.

## What Changes

- **T7001**: Expand CLI command structure with proper argument handling
- **T7002**: Implement run command to execute happy-path OpenSpec workflow
- **T7003**: Implement inspect artifacts command
- **T7004**: Implement inspect journal command

## Capabilities

### New Capabilities

- `cli-run-command`: Execute workflow from user prompt via CLI
- `cli-artifact-inspection`: List and display artifacts for a run
- `cli-journal-inspection`: Display journal events for a run

### Modified Capabilities

- (none - new capabilities only)

## Impact

- **Code affected**: `cli` crate - expanded command implementations
- **Dependencies**: Requires Phase 6 completion (workflow execution)
- **User-facing**: First interface for running the orchestrator
