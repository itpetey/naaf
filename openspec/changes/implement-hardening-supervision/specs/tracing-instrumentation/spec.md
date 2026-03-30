## ADDED Requirements

### Requirement: Run creation is instrumented
The system SHALL add a tracing span when a run is created.

#### Scenario: Run created
- **WHEN** a new Run is created
- **THEN** a tracing span is entered with run_id

### Requirement: Each transition is instrumented
The system SHALL add tracing spans for each transition execution.

#### Scenario: Transition executes
- **WHEN** execute_transition is called
- **THEN** a span is entered with transition name and phase

### Requirement: Workflow completion is instrumented
The system SHALL record when a workflow completes.

#### Scenario: Workflow ends
- **WHEN** run_workflow returns
- **THEN** a span is closed with outcome (accepted/escalated/failed)

### Requirement: Tracing includes key metadata
Spans SHALL include relevant context.

#### Scenario: Span metadata
- **WHEN** a span is created
- **THEN** it includes: run_id, phase, transition_name
