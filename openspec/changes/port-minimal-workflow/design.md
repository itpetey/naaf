## Context

This is the proving ground for the new architecture. The workflow must handle all three input classes.

## Goals / Non-Goals

**Goals:**
- Implement draft_request workflow matching REFACTOR_PLAN.md example
- Test all three input classes
- Prove end-to-end execution

**Non-Goals:**
- Complex workflows (that's later)
- Full domain coverage

## Decisions

1. **Workflow structure**
   - Decision: Match REFACTOR_PLAN.md target shape
   - Rationale: Already designed for this use case

2. **Testing approach**
   - Decision: Unit tests for each path
   - Rationale: Verify behavior without full LLM

## Risks / Trade-offs

- [Risk] LLM integration → [Mitigation] Mock for initial testing
