## ADDED Requirements

### Requirement: ExecutionEvent enum
The system SHALL provide an ExecutionEvent enum with variants: RunStarted, StepEntered, PromptRendered, ProviderCalled, ProviderResponded, ArtifactsParsed, ValidatorPassed, ValidatorFailed, RouteSelected, BranchStarted, BranchCompleted, JoinReduced, RunTerminated, RunFailed.

#### Scenario: Event emission
- **WHEN** execution progresses through steps
- **THEN** relevant events are emitted

### Requirement: Event contains required fields
The system SHALL ensure each event includes: run_id, state_id, step_name, timestamps.

#### Scenario: Event fields
- **WHEN** examining an event
- **THEN** it contains all required fields
