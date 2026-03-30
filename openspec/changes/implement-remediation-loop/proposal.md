## Why

Phase 8 defined the review and remediation workers, but they haven't been wired into an executable loop. We need to implement the bounded remediation loop that executes review workers, persists findings, applies targeted remediation, and decides whether to accept, iterate, or escalate. This completes the workflow execution pattern.

## What Changes

- **T9001**: Add review transition execution support (run review workers, persist findings)
- **T9002**: Add remediation planning and patch execution path (plan + apply one fix)
- **T9003**: Add retry/escalation policy for remediation loop (budget limits, escalation triggers)

## Capabilities

### New Capabilities

- `review-execution`: Execute review workers and persist findings
- `remediation-execution`: Plan and apply targeted remediation
- `remediation-loop`: Bounded loop with retry budget and escalation

### Modified Capabilities

- (none - new capabilities only)

## Impact

- **Code affected**: `orchestrator` crate - new execution paths for review/remediation
- **Dependencies**: Requires Phase 8 workers to be implemented
- **Workflow change**: Adds review -> remediation cycle after initial proposal
