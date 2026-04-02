## ADDED Requirements

### Requirement: ExecutionEvent enum
The system SHALL provide an ExecutionEvent enum with variants: RunStarted, StepEntered, PromptRendered, ProviderCalled, ProviderResponded, ArtifactsParsed, ValidatorPassed, ValidatorFailed, RouteSelected, BranchStarted, BranchCompleted, JoinReduced, RunTerminated, RunFailed.

#### Scenario: Event emission
- **WHEN** execution progresses through steps
- **THEN** relevant events are emitted

### Requirement: Event contains required fields
The system SHALL ensure each event includes: run_id, state_id, step_name, sequence_number, timestamp.

#### Scenario: Event fields
- **WHEN** examining an event
- **THEN** it contains all required fields

### Requirement: RunFailed event contains error information
The system SHALL ensure RunFailed event includes an error field with failuredetails.

#### Scenario: RunFailed event
- **WHEN** a workflow run fails
- **THEN** the RunFailed event contains error message

### Requirement: Events are sequentially ordered
The system SHALL provide monotonically increasing sequence numbers for event ordering.

#### Scenario: Event ordering
- **WHEN** events are emitted
- **THEN** each event has a unique sequence number that can be used to reconstruct execution order
