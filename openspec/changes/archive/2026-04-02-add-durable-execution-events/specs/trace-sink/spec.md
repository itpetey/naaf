## ADDED Requirements

### Requirement: TraceSink trait
The system SHALL provide a TraceSink trait with emit() method for event emission.

#### Scenario: Event emission
- **WHEN** calling trace_sink.emit(event)
- **THEN** the event is recorded
