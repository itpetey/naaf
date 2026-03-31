## Context

Phase 8 defined the review and remediation workers. Now we need to execute them as part of a bounded remediation loop. The loop pattern:

1. Run RiskReviewer + ConsistencyReviewer (parallel)
2. Aggregate findings
3. If findings exist: RemediationPlanner -> TargetedRemediator -> ReadinessEvaluator
4. Repeat until accepted or escalated

ARCHITECTURE.md specifies maximum 2 remediation loops for v1.

## Goals / Non-Goals

**Goals:**
- Execute review workers in parallel
- Persist findings to FindingStore
- Execute remediation planning and patching
- Implement retry budget and escalation triggers
- Support terminal outcomes: accepted, escalated, rejected

**Non-Goals:**
- Complex finding correlation across multiple runs
- Multiple simultaneous remediation paths
- Advanced retry strategies

## Decisions

### Decision 1: Review Execution Flow

Execute RiskReviewer and ConsistencyReviewer in parallel, then aggregate.

Rationale: ARCHITECTURE.md allows parallel review passes. Both can run simultaneously.

### Decision 2: Finding Persistence

Findings are persisted to FindingStore with status tracking.

Rationale: FindingStore implementation (T2003) should be complete. Use it here.

### Decision 3: Remediation Loop Structure

```
Loop (max 2 iterations):
  1. Run reviews -> findings
  2. If no findings: Accept
  3. Plan remediation
  4. If escalate: Escalate
  5. Apply patch
  6. Evaluate readiness
  7. If accepted: Accept
  8. If rejected: Reject
  9. Next iteration
```

Rationale: Simple bounded loop. Maximum 2 iterations per ARCHITECTURE.md.

### Decision 4: Retry Budget

Each transition has retry_limit from TransitionSpec. If exceeded, escalate.

Rationale: TransitionSpec already has retry_limit. Apply consistently.

### Decision 5: Escalation Triggers

Escalate when:
- Retry budget exceeded
- Same finding recurs twice
- RemediationPlanner recommends escalation
- Finding count exceeds threshold (>10)

Rationale: Clear, enforceable rules.

## Risks / Trade-offs

- [Risk] Loop may not terminate → [Mitigation] Hard limit on iterations
- [Risk] Same finding reappears → [Mitigation] Track finding IDs; escalate after 2nd occurrence
- [Risk] Scope drift → [Mitigation] TargetedRemediator constrains edits; detect if more than 1 section changes

## Migration Plan

1. Implement execute_review_transitions (T9001)
2. Implement execute_remediation_cycle (T9002)
3. Implement retry/escalation logic (T9003)
4. Wire into workflow execution

## Open Questions

- Should we track finding resolution history? → Decision: Simple status flag is sufficient for v1
