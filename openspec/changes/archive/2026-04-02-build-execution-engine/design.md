## Context

The executor is the core runtime component that runs compiled workflows. It must handle all step types, routing decisions, branches, joins, and budget enforcement.

## Goals / Non-Goals

**Goals:**
- Implement DAG walker for compiled graphs
- Handle all step types (Transformer, Router, Reducer, Validator)
- Implement budget enforcement
- Support cancellation
- Emit execution events

**Non-Goals:**
- Persist events (that's Phase 6)
- Implement specific step implementations (that's later)

## Decisions

1. **Execution model**
   - Decision: Use async/await with tokio
   - Rationale: Per AGENTS.md guidelines, enables concurrent branch execution

2. **Budget implementation**
   - Decision: Track step count, branch count, token count, time elapsed
   - Rationale: Per REFACTOR_PLAN.md requirements

3. **Services abstraction**
   - Decision: Use trait for services, allow runtime injection
   - Rationale: Flexibility for testing and different backends

## Risks / Trade-offs

- [Risk] Complex async flow → [Mitigation] Start with simple sequential execution
- [Low] May need refinement → Acceptable
