## Context

REFACTOR_PLAN.md specifies a canonical state envelope shape. Currently, the prototype has different runtime structs per workflow state. We need a unified approach.

## Goals / Non-Goals

**Goals:**
- Define `StateEnvelope` with all required fields
- Create supporting types for IDs, artifacts, metadata, lineage
- Implement serde serialization
- Keep state immutable

**Non-Goals:**
- Implement validation logic (that's later)
- Define step traits (Phase 3)
- Handle persistence (Phase 6)

## Decisions

1. **ID types**
   - Decision: Use `uuid::Uuid` for both `StateId` and `RunId`
   - Rationale: Unique, sortable, widely used

2. **ArtifactValue enum**
   - Decision: Include both generic variants (Text, Json) and domain-specific (NormalizedRequest, ScopeDoc, etc.)
   - Rationale: Flexibility with type safety

3. **StateKind not mixed with execution status**
   - Decision: Keep StateKind semantic-only, handle execution status separately
   - Rationale: Per design principle #8 in REFACTOR_PLAN.md

## Risks / Trade-offs

- [Risk] Complex enum with many variants → [Mitigation] Start with core variants, add incrementally
- [Low] Serialization complexity → Acceptable, needed for persistence
