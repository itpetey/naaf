## ADDED Requirements

### Requirement: TraceSink trait
The system SHALL provide a TraceSink trait with emit() method for event emission.

#### Scenario: Event emission
- **WHEN** calling trace_sink.emit(event)
- **THEN** the event is recorded

### Requirement: AsyncTraceSink trait
The system SHALL provide an AsyncTraceSink trait for asynchronous event emission.

#### Scenario: Async event emission
- **WHEN** calling async_trace_sink.emit(event).await
- **THEN** the event is recorded asynchronously

### Requirement: Error handling
The system SHALL ensure emit() returns EventResult to propagate errors.

#### Scenario: Error propagation
- **WHEN** event emission fails
- **THEN** the error is returned to the caller

### Requirement: NoOp implementations
The system SHALL provide NoOpTraceSink and NoOpAsyncTraceSink for testing scenarios.

#### Scenario: Testing
- **WHEN** using NoOp implementations
- **THEN** events are discarded without error
