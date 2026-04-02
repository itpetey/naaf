## Context

The REFACTOR_PLAN.md specifies a new crate structure with clear separation of concerns. Currently, the codebase has runtime logic mixed with application code. We need to introduce new crates to provide a clean foundation for the workflow runtime.

## Goals / Non-Goals

**Goals:**
- Create 4 new crates with minimal scaffolding
- Define module structure per REFACTOR_PLAN.md skeleton
- Wire crates into workspace dependencies
- Use workspace dependencies to avoid version conflicts

**Non-Goals:**
- Implement any runtime functionality yet
- Remove or modify existing legacy code
- Define state or step traits (that's Phase 2-3)

## Decisions

1. **Crate dependencies**
   - Decision: Use `[workspace.dependencies]` in root `Cargo.toml`
   - Rationale: Prevents version conflicts, ensures consistent versions

2. **Edition**
   - Decision: Use Rust Edition 2024
   - Rationale: Per AGENTS.md requirements

3. **Initial module structure**
   - Decision: Create empty modules matching skeleton
   - Rationale: Establishes structure without functionality

## Risks / Trade-offs

- [Risk] Breaking workspace build → [Mitigation] Keep crates minimal, test build incrementally
- [Low] Module structure may change → Acceptable, can refactor later
