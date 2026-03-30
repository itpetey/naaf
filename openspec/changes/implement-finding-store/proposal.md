## Why

The orchestrator currently records findings only through journal events (FindingCreated, FindingResolved) but lacks a dedicated finding store. This prevents efficient querying, updating, and persistence of findings across runs. Without a proper finding store, we cannot properly track remediation progress or maintain finding history.

## What Changes

- Implement a dedicated `FindingStore` in the orchestrator's store module
- Add save/load/list/delete operations for findings
- Add tests for finding persistence
- Update the journal to reference persisted finding IDs for better traceability

## Capabilities

### New Capabilities

- `finding-store`: A dedicated filesystem-backed store for persisting and querying structured findings

### Modified Capabilities

- (none - this is a new capability supporting existing finding model)

## Impact

- **Code affected**: `orchestrator` crate - new store module additions
- **Dependencies**: Uses existing Finding model (T1004), run directory layout (T2001)
- **Storage**: New directory structure under run paths for finding persistence
