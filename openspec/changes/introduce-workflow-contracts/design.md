## Context

Workflow composition requires contract-based interfaces rather than structural assumptions.

## Goals / Non-Goals

**Goals:**
- Define WorkflowContract struct
- Implement contract validation
- Support adapters

**Non-Goals:**
- Implement full orchestration engine
- Handle complex graph composition

## Decisions

1. **Contract fields**
   - Decision: accepted_kinds, required_artifacts, guaranteed_artifacts, possible_output_kinds
   - Rationale: Covers input requirements and output guarantees

2. **Adapter approach**
   - Decision: Simple transform function
   - Rationale: Sufficient for basic reshaping

## Risks / Trade-offs

- [Low] Contract complexity → Acceptable for v1
