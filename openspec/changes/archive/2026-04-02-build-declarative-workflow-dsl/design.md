## Context

REFACTOR_PLAN.md specifies a builder API target shape. Workflows must be declarative and validated before execution. This is Phase 4 of 13.

## Goals / Non-Goals

**Goals:**
- Create builder API matching REFACTOR_PLAN.md target shape
- Implement compile-time validation
- Produce valid compiled graph

**Non-Goals:**
- Execute workflows (that's Phase 5)
- Handle persistence (that's Phase 6)

## Decisions

1. **Builder API style**
   - Decision: Use builder pattern with method chaining
   - Rationale: Matches target shape in REFACTOR_PLAN.md

2. **Graph representation**
   - Decision: Use adjacency list with explicit edge types
   - Rationale: Simple, debuggable, sufficient for DAGs

3. **Validation phase**
   - Decision: Separate compile() method that validates and returns CompiledWorkflow
   - Rationale: Fail fast before execution

## Risks / Trade-offs

- [Risk] Complex validation logic → [Mitigation] Validate incrementally during build
- [Low] Builder API may need refinement → Acceptable, can refactor
