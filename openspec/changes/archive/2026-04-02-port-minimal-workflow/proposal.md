## Why

We need to prove the new runtime works by implementing one minimal workflow end-to-end. This workflow demonstrates all required capabilities: transformer, router, ambiguity handling, escalation/clarification, terminal, and durable trace.

## What Changes

- Create `draft_request` workflow using WorkflowBuilder
- Implement steps: propose, classify_input, normalize, scope, plan, accept, terminal
- Add router for initial decision (greeting/clarify/continue)
- Add branch paths for each decision
- Add tests for happy path and ambiguous path
- Ensure all three input classes work: "Hi", "Help me improve this", actionable request

## Capabilities

### New Capabilities
- `draft-request-workflow`: Full workflow implementation

### Modified Capabilities
- (none yet)

## Impact

- First production workflow in new runtime
- Proof of architecture
