## Why

The app layer must be updated to run the new workflow runtime. The interface should support inspection, not just execution.

## What Changes

- Update CLI to run new workflow runtime
- Add commands: run workflow, show run trace, inspect final state, replay run, list workflows
- Update TUI to display: current step, state kind, key artifacts, route decisions, validation failures, branch status, final output

## Capabilities

### New Capabilities
- `cli-commands`: CLI commands for new runtime
- `tui-display`: TUI for workflow inspection

### Modified Capabilities
- (none yet)

## Impact

- Updated `workflow-app` crate
- User-facing functionality
