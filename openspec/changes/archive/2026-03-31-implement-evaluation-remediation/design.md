## Context

ARCHITECTURE.md defines a bounded remediation loop pattern:
1. Run validators and reviewers
2. Produce structured findings
3. Rank findings by severity
4. Select one finding or cluster
5. Generate targeted remediation plan
6. Limit edit scope
7. Apply patch
8. Re-run checks
9. Stop on success or escalate

Phase 8 implements all the workers that enable this loop:
- RiskReviewer and ConsistencyReviewer (produce findings)
- FindingsAggregator (merge and prioritize)
- RemediationPlanner (select next fix)
- TargetedRemediator (apply narrow fix)
- ReadinessEvaluator (decide accept/escalate)

## Goals / Non-Goals

**Goals:**
- Implement all review workers that emit structured findings
- Implement remediation workers that constrain edit scope
- Enable the bounded remediation loop
- Support escalation as a valid outcome

**Non-Goals:**
- Actual execution of review/remediation (Phase 9)
- Complex finding correlation algorithms
- Multiple simultaneous remediation paths

## Decisions

### Decision 1: Finding Payload Types

Create OpenSpec-specific finding payload types that map to orchestrator Finding:
- RiskFinding: id, category, severity, evidence, impacted_section, mitigation
- ConsistencyFinding: id, category, severity, quoted_evidence, impacted_sections

Rationale: Keep OpenSpec-specific detail in openspec crate; map to generic Finding for orchestrator.

### Decision 2: Review Workers are Non-Mutating

RiskReviewer and ConsistencyReviewer only produce findings, never modify proposal.

Rationale: Clear separation of concerns. Review != edit.

### Decision 3: FindingsAggregator Output

Produces a unified FindingSet sorted by priority (severity + dependency order).

Rationale: Single prioritized queue for remediation.

### Decision 4: RemediationPlanner Selection

Selects one finding or tightly related cluster. Maximum scope: same section.

Rationale: Enforces narrow edits per iteration.

### Decision 5: TargetedRemediator Output

Returns SectionPatch with target sections, replacement text, rationale.

Rationale: Minimal output for applying fix.

### Decision 6: ReadinessEvaluator Decision

Returns: accepted | escalated | rejected with reasons.

Rationale: Explicit decision with traceability.

## Risks / Trade-offs

- [Risk] Finding prioritization is subjective → [Mitigation] Use severity + section as heuristic
- [Risk] Related cluster detection is complex → [Decision] Simple same-section grouping for v1

## Migration Plan

1. Add finding payload types (T8001)
2. Implement RiskReviewer spec + prompt (T8002)
3. Implement ConsistencyReviewer spec + prompt (T8003)
4. Implement FindingsAggregator spec (T8004)
5. Implement RemediationPlanner spec (T8005)
6. Implement TargetedRemediator spec (T8006)
7. Implement ReadinessEvaluator spec (T8007)

## Open Questions

- Should we support automatic retry of same finding? → Decision: No; escalate after failed remediation
