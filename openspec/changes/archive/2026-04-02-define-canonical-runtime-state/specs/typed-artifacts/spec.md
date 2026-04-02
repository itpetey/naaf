## ADDED Requirements

### Requirement: ArtifactMap storage
The system SHALL provide an `ArtifactMap` type for storing workflow artifacts as key-value pairs.

#### Scenario: Adding artifacts
- **WHEN** adding an artifact to ArtifactMap
- **THEN** the artifact is stored and retrievable by key

### Requirement: ArtifactKey type
The system SHALL provide an `ArtifactKey` type for uniquely identifying artifacts within a state.

#### Scenario: Artifact key creation
- **WHEN** creating an ArtifactKey
- **THEN** the key can identify artifacts across state transitions

### Requirement: ArtifactValue enum
The system SHALL provide an `ArtifactValue` enum supporting: Text(String), Json(serde_json::Value), and domain-specific variants.

#### Scenario: Artifact value storage
- **WHEN** storing artifacts with different types
- **THEN** each type is correctly preserved in ArtifactValue
