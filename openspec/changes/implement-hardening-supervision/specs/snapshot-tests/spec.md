## ADDED Requirements

### Requirement: Artifact serialization is tested
The system SHALL have tests verifying artifacts serialize/deserialize correctly.

#### Scenario: Artifact round-trip
- **GIVEN** a NormalizedSpec
- **WHEN** it is serialized and deserialized
- **THEN** all fields match the original

### Requirement: Journal events are tested
The system SHALL have tests for journal event format.

#### Scenario: Journal event format
- **GIVEN** a TransitionExecuted event
- **WHEN** it is serialized to JSON
- **THEN** the output matches expected format

### Requirement: Workflow graph construction is tested
The system SHALL have tests verifying workflow definition builds correctly.

#### Scenario: Happy-path workflow
- **WHEN** openspec_happy_path() is called
- **THEN** it returns a valid workflow with 4 phases

### Requirement: Store operations are tested
The system SHALL have tests for artifact and finding stores.

#### Scenario: Artifact store round-trip
- **GIVEN** an artifact saved to store
- **WHEN** it is loaded
- **THEN** content matches original
