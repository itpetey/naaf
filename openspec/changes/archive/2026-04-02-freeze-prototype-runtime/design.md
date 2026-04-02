## Context

The NAAF project currently has a working but limited prototype runtime. This prototype uses a linear artifact pipeline model that cannot support the required features from REFACTOR_PLAN.md. Rather than continuing to evolve this prototype, we will freeze it as legacy and build a new runtime.

## Goals / Non-Goals

**Goals:**
- Preserve the current prototype code for reference and rollback
- Clearly mark the legacy status to prevent further investment in the old architecture
- Establish migration policy for contributors
- Create infrastructure for the upcoming workflow runtime (Phase 1+)

**Non-Goals:**
- Delete or modify any existing prototype code
- Implement any new runtime features in this phase
- Migrate existing workflows (that's Phase 11+)

## Decisions

1. **Git branch vs tag for legacy code**
   - Decision: Use both a branch and a tag
   - Rationale: Branch allows continued reference; tag marks exact version

2. **Documentation location**
   - Decision: Add LEGACY.md at repository root, add migration notes to existing README.md
   - Rationale: Central location for future contributors to understand context

3. **Keep prototype code in main**
   - Decision: Do not move prototype to separate directory
   - Rationale: Moving code creates merge complexity; easier to just mark as legacy and build new beside it

## Risks / Trade-offs

- [Risk] Contributors continue using old runtime → [Mitigation] Clear documentation and README updates
- [Risk] Confusion about what's legacy vs new → [Mitigation] Clear naming conventions and documentation
- [Low] No actual code changes, only documentation → Acceptable for this phase
