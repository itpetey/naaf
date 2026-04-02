## ADDED Requirements

### Requirement: TryFromState trait
The system SHALL provide a TryFromState trait with method `fn try_from_state(key: &ArtifactKey, state: &StateEnvelope) -> Result<Self, AdapterError>` where Self: Sized.

#### Scenario: Typed extraction
- **WHEN** calling try_from_state on a type with an artifact key
- **THEN** it returns the typed value from that artifact or error

#### Scenario: Multiple artifacts
- **GIVEN** a state with multiple artifacts
- **WHEN** extracting different artefacts by key
- **THEN** each extraction works independently

### Requirement: IntoState trait
The system SHALL provide an IntoState trait with method `fn into_state(self, key: ArtifactKey, state: &mut StateEnvelope)`.

#### Scenario: Typed insertion
- **WHEN** calling into_state with a value and key
- **THEN** the value is stored in the state under that key

### Requirement: AdapterError type
The system SHALL provide an AdapterError enum in workflow-schema with variants for missing artifacts, type mismatches, and JSON errors.

#### Scenario: Missing artifact
- **WHEN** extracting from a non-existent artifact key
- **THEN** it returns MissingArtifact error

#### Scenario: Type mismatch
- **WHEN** extracting a value that doesn't match the expected type
- **THEN** it returns TypeMismatch error
