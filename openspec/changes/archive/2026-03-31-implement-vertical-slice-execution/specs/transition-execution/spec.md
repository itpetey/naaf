## ADDED Requirements

### Requirement: execute_transition loads required artifacts
The system SHALL load input artifacts before executing a transition.

#### Scenario: Required artifact exists
- **GIVEN** a TransitionSpec requiring ArtifactKind::UserPrompt
- **AND** the artifact exists in the store
- **WHEN** execute_transition() is called
- **THEN** the artifact content is loaded

#### Scenario: Required artifact missing
- **GIVEN** a TransitionSpec requiring an artifact that doesn't exist
- **WHEN** execute_transition() is called
- **THEN** EngineError::MissingArtifact is returned

### Requirement: execute_transition saves produced artifact
The system SHALL persist the new artifact after successful execution.

#### Scenario: Transition succeeds
- **GIVEN** a successful LLM call and decoded output
- **WHEN** execute_transition() completes
- **THEN** the new artifact is saved to the store

### Requirement: execute_transition updates run phase
The system SHALL advance the run to the next phase.

#### Scenario: Transition from Proposed to Normalized
- **GIVEN** a run at Phase::Proposed
- **AND** a transition to Phase::Normalized
- **WHEN** execute_transition() completes
- **THEN** the run's phase is updated to Phase::Normalized

### Requirement: execute_transition records journal event
The system SHALL log the transition execution.

#### Scenario: Transition executed
- **WHEN** execute_transition() is called
- **THEN** a TransitionExecuted journal event is recorded

### Requirement: execute_transition handles failures
The system SHALL handle and propagate errors appropriately.

#### Scenario: LLM call fails
- **GIVEN** ModelProvider returns an error
- **WHEN** execute_transition() is called
- **THEN** EngineError::WorkerFailed is returned
- **AND** journal records the failure
