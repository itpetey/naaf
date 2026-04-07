## Why

The CLI can detect that a request is ambiguous, but it cannot collect clarification from the user in the same session. Today `naaf run` prints an escalation and stops, which forces the user to manually rewrite and rerun the request.

The smallest useful improvement is to let the CLI ask for one clarification when an ambiguous escalation is returned.

## What Changes

- Detect ambiguous escalation results in `naaf run`
- Prompt for one clarification when running interactively in a terminal
- Re-run the workflow with the original input plus clarification as a new run
- Keep current non-interactive behaviour unchanged

## Capabilities

### New Capabilities

### Modified Capabilities
- `cli-commands`: `run` can collect clarification for ambiguous requests during an interactive session

## Impact

- **Code affected**: `crates/cli/src/main.rs`
- **Dependencies**: No workflow runtime or schema changes required
- **User-facing**: Users can disambiguate requests without leaving the CLI session
