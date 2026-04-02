## 1. Define ID types

- [x] 1.1 Create `workflow-schema/src/state.rs`
- [x] 1.2 Define `StateId` and `RunId` as newtype wrappers around uuid::Uuid
- [x] 1.3 Implement Display, Debug, Clone, Copy, PartialEq, Eq, Hash for IDs
- [x] 1.4 Add Serialize, Deserialize implementations

## 2. Define StateKind and Lineage

- [x] 2.1 Define `StateKind` enum with variants: Proposed, Normalized, Scoped, Planned, Accepted, Ambiguous, Escalated, Terminal
- [x] 2.2 Define `Lineage` struct with parent_state_id and transition info
- [x] 2.3 Define `StateMeta` struct with created_at timestamp
- [x] 2.4 Add Serialize, Deserialize for all types

## 3. Define Artifact types

- [x] 3.1 Create `workflow-schema/src/artifacts.rs`
- [x] 3.2 Define `ArtifactKey` newtype
- [x] 3.3 Define `ArtifactValue` enum with Text, Json, and domain variants
- [x] 3.4 Define `ArtifactMap` as HashMap<ArtifactKey, ArtifactValue>
- [x] 3.5 Add typed accessor helpers

## 4. Define StateEnvelope

- [x] 4.1 Define `StateEnvelope` struct with all required fields
- [x] 4.2 Implement `new()` constructor
- [x] 4.3 Add Serialize, Deserialize implementations
- [x] 4.4 Write unit tests

## 5. Verify build

- [x] 5.1 Run `cargo build -p workflow-schema`
- [x] 5.2 Fix any compilation errors
