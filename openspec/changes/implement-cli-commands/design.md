## Context

The CLI crate exists with a basic clap-based skeleton (main.rs) but the commands are stubs. The orchestrator can now execute workflows (Phase 6), so we need to wire the CLI to actually:
- Create runs from user prompts
- Execute the happy-path workflow
- Inspect persisted artifacts
- Inspect journal events

## Goals / Non-Goals

**Goals:**
- Implement working run command that executes the full workflow
- Implement artifact inspection with listing and viewing
- Implement journal inspection for debugging
- Show clear output and run location

**Non-Goals:**
- Interactive prompts or TUI
- Multiple workflow types (happy-path only for v1)
- Resume/abort commands

## Decisions

### Decision 1: CLI Structure

Use clap with subcommands:
- `naaf run <prompt>` - Execute workflow
- `naaf list` - List runs
- `naaf inspect <run-id>` - Show run details
- `naaf artifacts <run-id>` - List artifacts
- `naaf journal <run-id>` - Show journal

Rationale: Simple, familiar CLI patterns. Subcommands scale well.

### Decision 2: Run Command Flow

1. Parse prompt from args
2. Create UserPrompt artifact
3. Initialize run with RunId
4. Execute workflow via run_workflow()
5. Report outcome and artifact location

Rationale: Single command handles the full flow.

### Decision 3: Artifact Listing Format

Show: ID, Kind, Created At
Allow: `--json` flag for machine-readable output

Rationale: Human-readable default, JSON for scripting.

### Decision 4: Journal Display

Show each event on separate line with timestamp and event type.

Rationale: Simple, grep-friendly format.

### Decision 5: Run Storage Location

Use `.runs/{run_id}/` directory under current working directory.

Rationale: Easy to find, separate from source code.

## Risks / Trade-offs

- [Risk] Long-running workflows → [Decision] Show progress, allow Ctrl+C
- [Risk] API key not set → [Decision] Show clear error message with setup instructions
- [Risk] Large output → [Decision] Paginate or truncate with --verbose flag

## Migration Plan

1. Add dependencies (orchestrator, openspec, provider-openai) to cli Cargo.toml
2. Implement run command with workflow execution
3. Implement artifacts command with store access
4. Implement journal command with journal reader
5. Test end-to-end

## Open Questions

- Should we add provider selection flag? → Deferred to v2; default to OpenAI
- Should we support config file for defaults? → Deferred; env vars work for now
