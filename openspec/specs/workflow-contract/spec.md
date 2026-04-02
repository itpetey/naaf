## ADDED Requirements

### Requirement: WorkflowContract structure
The system SHALL provide a WorkflowContract with fields: accepted_kinds (Vec<StateKind>), required_artifacts (Vec<ArtifactKey>), guaranteed_artifacts (Vec<ArtifactKey>), possible_output_kinds (Vec<StateKind>).

#### Scenario: Contract definition
- **WHEN** defining a workflow contract
- **THEN** all contract fields are specified
