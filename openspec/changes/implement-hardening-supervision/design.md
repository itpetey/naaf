## Context

Phase 10 is the final hardening phase. We've built the core workflow engine through Phases 1-9. Now we need:
1. Tracing to debug issues in production
2. Tests to prevent regressions
3. A TUI backlog documenting future work

According to ARCHITECTURE.md: "Focus on observability, not telemetry maximalism."

## Goals / Non-Goals

**Goals:**
- Add tracing spans at major run/transition boundaries
- Add deterministic tests for happy-path behavior
- Document TUI backlog for future implementation

**Non-Goals:**
- Full telemetry pipeline
- Performance benchmarking
- Implementing TUI (document only)

## Decisions

### Decision 1: Tracing Approach

Use `tracing` crate with spans at:
- Run creation
- Each transition execution
- Workflow completion

Rationale: Simple, practical. Matches AGENTS.md preference for tracing.

### Decision 2: Test Strategy

Use snapshot tests for:
- Artifact serialization/deserialization
- Journal event output
- Workflow graph construction

Rationale: Catches structural changes. Easy to verify.

### Decision 3: TUI Backlog Format

Create a markdown document with:
- Feature name
- Priority (high/medium/low)
- Description
- Dependencies

Rationale: Simple, easy to update. Lives in docs/.

### Decision 4: Test Fixtures

Use tempfile for filesystem tests.
Use mock providers for LLM calls.

Rationale: Deterministic, no external dependencies.

## Risks / Trade-offs

- [Risk] Snapshot tests may drift → [Mitigation] Review snapshots on failure; update intentionally
- [Risk] TUI backlog becomes stale → [Decision] Review quarterly

## Migration Plan

1. Add tracing to Cargo.toml (already present per BOOTSTRAP_PACK.md)
2. Add spans to key functions
3. Add snapshot tests
4. Create TUI backlog document

## Open Questions

- Should we add performance tests? → Deferred to v2
