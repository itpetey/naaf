## Context

The orchestrator currently records findings only through journal events (FindingCreated, FindingResolved) in journal.rs. However, this approach has limitations:

1. Findings are only accessible by replaying journal events
2. No efficient way to query findings by status, severity, or category
3. Finding history is not preserved in a queryable form
4. Cannot easily update finding status without re-processing journal

The existing ArtifactStore (store.rs) provides a proven pattern for filesystem-backed persistence that we can reuse.

## Goals / Non-Goals

**Goals:**
- Implement a FindingStore following the same patterns as ArtifactStore
- Support save/load/list/delete operations for findings
- Enable querying findings by status, severity, and run
- Maintain compatibility with existing Finding model (T1004)

**Non-Goals:**
- Database-backed storage (filesystem-only for v1)
- Complex querying or indexing beyond basic filters
- Distributed finding aggregation across runs

## Decisions

### Decision 1: Directory Structure

Use `{root}/findings/{run_id}/{finding_id}.json` for finding persistence.

Rationale: Follows the same pattern as ArtifactStore for consistency. The `findings` subdirectory mirrors `artifacts`.

### Decision 2: FindingStore API

Expose these methods:
- `save(&self, finding: &Finding) -> StoreResult<()>`
- `load(&self, id: FindingId, run_id: RunId) -> StoreResult<Finding>`
- `list(&self, run_id: RunId) -> StoreResult<Vec<Finding>>`
- `list_by_status(&self, run_id: RunId, status: FindingStatus) -> StoreResult<Vec<Finding>>`
- `update_status(&self, id: FindingId, run_id: RunId, status: FindingStatus) -> StoreResult<()>`
- `delete(&self, id: FindingId, run_id: RunId) -> StoreResult<()>`
- `delete_run(&self, run_id: RunId) -> StoreResult<()>`

Rationale: Mirrors ArtifactStore API with additional status-based querying since findings have lifecycle states.

### Decision 3: Serialization Format

Store findings as JSON files (not binary), unlike artifacts which use binary content.

Rationale: Findings are structured data, not large payloads. JSON is human-readable and easier to debug. Aligns with metadata handling in ArtifactStore.

### Decision 4: Journal Integration

Update journal events to include `finding_id: FindingId` for traceability.

Rationale: Allows linking journal events to persisted findings. FindingCreated event should store the finding ID for later reference.

## Risks / Trade-offs

- [Risk] Duplicate finding IDs between runs → [Mitigation] Use run_id as directory prefix, finding ID is unique within run context
- [Risk] Large number of findings per run → [Mitigation] JSON per file is fine for v1; consider indexed approach later if needed
- [Finding] No transaction support → [Accept] Single-file operations are atomic at filesystem level; no multi-finding atomicity needed for v1

## Migration Plan

1. Add FindingStore struct to store.rs alongside ArtifactStore
2. Add unit tests following existing store.rs test patterns
3. Update journal.rs to record finding_id in FindingCreated/FindingResolved events
4. Update any existing code that creates findings to persist via FindingStore

## Open Questions

- Should FindingStore be initialized per-run or as a shared singleton? → Decision: Per-run, matching ArtifactStore pattern
- Should we add a FindingStoreError type or reuse StoreError? → Decision: Extend StoreError with Finding-specific variants
