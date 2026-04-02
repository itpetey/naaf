## ADDED Requirements

### Requirement: Lineage tracking
The system SHALL provide a `Lineage` type that tracks state transitions for debugging and replay.

#### Scenario: State lineage
- **WHEN** creating a new state from a previous state
- **THEN** the lineage records the parent state ID and transition metadata

### Requirement: StateMeta
The system SHALL provide a `StateMeta` type for storing metadata about state creation including timestamps.

#### Scenario: Metadata capture
- **WHEN** creating a new state
- **THEN** StateMeta captures creation timestamp and other relevant metadata
