## Context

`naaf run` currently accepts a single input, executes the workflow, prints final artifacts, and exits. Ambiguous requests are routed to a clarification terminal that records escalation metadata, but there is no way for the CLI to ask the user for follow-up input.

The runtime does not currently support pausing and resuming a run after human input. The smallest change should avoid adding that infrastructure.

## Goals / Non-Goals

**Goals:**
- Let users provide one clarification in the same CLI session
- Keep the implementation local to the CLI crate
- Preserve existing behaviour for non-interactive usage

**Non-Goals:**
- Resume the same run ID after human input
- Add generic human-in-the-loop workflow support
- Add TUI-based run control
- Introduce multi-turn clarification loops

## Decisions

### Decision 1: Implement clarification handling in the CLI only

When `run` completes with an `escalation` artifact whose `classification` is `Ambiguous`, the CLI will decide whether to prompt for clarification.

Rationale: The current workflow already signals ambiguity. The missing piece is only user input collection.

### Decision 2: Only prompt in interactive terminals

The CLI should only prompt when stdin and stdout are terminals. In non-interactive contexts, it should preserve the current output and exit normally.

Rationale: This keeps scripts and pipes deterministic.

### Decision 3: Clarification starts a new run

After collecting clarification, the CLI will compose a clarified input from:
1. the original input
2. the user clarification

It will then execute `run` again as a new run.

Rationale: This avoids adding pause/resume semantics to the runtime and keeps the change small.

### Decision 4: Allow one clarification attempt

The CLI should prompt once. If the second run is still ambiguous, it should print the escalation and stop.

Rationale: One retry provides value without creating an open-ended conversation loop.

### Decision 5: Make the retry explicit in output

The CLI should print that the original run was ambiguous and that it is starting a new clarified run, including both run IDs.

Rationale: Users need to understand that this is a follow-up run, not an in-place resume.

## Risks / Trade-offs

- [Risk] Users may expect the same run to continue
  - [Decision] Print explicit messaging that clarification creates a new run
- [Risk] Clarified input may still classify as ambiguous
  - [Decision] Stop after one clarification attempt
- [Risk] Interactive prompting could break automation
  - [Decision] Only prompt when attached to a terminal

## Migration Plan

1. Detect ambiguous escalation in `run`
2. Add terminal detection and stdin prompt
3. Re-run with composed clarified input as a new run
4. Update output messaging
5. Verify interactive and non-interactive behaviour

## Open Questions

- Should the clarified input be formatted as a structured block (`Original request` / `Clarification`) or appended as plain text?
  - Recommendation: use a structured block for predictable behaviour and clearer debugging.
