## Context

REFACTOR_PLAN.md specifies four core step traits. These are the foundation for the workflow builder and executor. The traits must be stateless and composable.

## Goals / Non-Goals

**Goals:**
- Define four step traits per REFACTOR_PLAN.md specifications
- Define shared error types
- Create boxed wrapper types for runtime use
- Make traits object-safe where useful

**Non-Goals:**
- Implement any concrete step instances (that's later)
- Handle execution context yet (that's Phase 5)

## Decisions

1. **Trait bounds**
   - Decision: `Send + Sync` for all traits
   - Rationale: Enables safe concurrent execution

2. **Error handling**
   - Decision: Use `thiserror` for error types
   - Rationale: Per AGENTS.md guidelines

3. **RouteDecision type**
   - Decision: Include variants for single next step, branch, or terminal
   - Rationale: Matches graph structure requirements

## Risks / Trade-offs

- [Low] Trait design may evolve → Acceptable as we learn more
