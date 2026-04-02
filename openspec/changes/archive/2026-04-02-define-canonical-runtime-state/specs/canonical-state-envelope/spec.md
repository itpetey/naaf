## ADDED Requirements

### Requirement: StateEnvelope structure
The system SHALL provide a `StateEnvelope` struct with fields: `id: StateId`, `run_id: RunId`, `kind: StateKind`, `artifacts: ArtifactMap`, `meta: StateMeta`, `lineage: Lineage`.

#### Scenario: StateEnvelope creation
- **WHEN** creating a new StateEnvelope
- **THEN** all required fields are populated with valid values

### Requirement: StateId and RunId types
The system SHALL provide `StateId` and `RunId` types based on UUID for unique identification.

#### Scenario: ID uniqueness
- **WHEN** generating StateId and RunId
- **THEN** each ID is unique across runs

### Requirement: StateKind enum
The system SHALL provide a `StateKind` enum with variants: Proposed, Normalized, Scoped, Planned, Accepted, Ambiguous, Escalated, Terminal.

#### Scenario: StateKind usage
- **WHEN** checking StateKind
- **THEN** the variant accurately reflects the semantic state
