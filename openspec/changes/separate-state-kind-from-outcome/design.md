## Context

StateKind currently mixes semantic workflow stages with execution status. This violates REFACTOR_PLAN.md design principle #8.

## Goals / Non-Goals

**Goals:**
- Separate StateKind (semantic) from ExecutionStatus (runtime) and WorkflowOutcome (final)
- Ensure clean semantic model

**Non-Goals:**
- Change step traits (already done)
- Change executor logic

## Decisions

1. **ExecutionStatus**
   - Decision: Pending, Running, Succeeded, Failed
   - Rationale: Standard execution states

2. **WorkflowOutcome**
   - Decision: Completed, NeedHumanClarification, Rejected, Escalated, Aborted
   - Rationale: Covers all terminal outcomes

## Risks / Trade-offs

- [Low] May need more variants → Acceptable, can extend
