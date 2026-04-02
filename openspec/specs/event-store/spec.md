## ADDED Requirements

### Requirement: FilesystemEventStore
The system SHALL provide a FilesystemEventStore that persists events to filesystem.

#### Scenario: Event persistence
- **WHEN** writing events to FilesystemEventStore
- **THEN** events are stored persistently

### Requirement: JSON lines format
The system SHALL store events in JSON lines format (one JSON object per line).

#### Scenario: Event format
- **WHEN** reading event log
- **THEN** events can be parsed line by line

### Requirement: Event replay
The system SHALL provide utilities to read and filter events from storage.

#### Scenario: Event replay
- **WHEN** calling read_events()
- **THEN** all events are returned in order

#### Scenario: Event filtering
- **WHEN** calling read_events_by_run()
- **THEN** only events for specified run are returned

### Requirement: Error handling
The system SHALL return Result types for all storage operations.

#### Scenario: Storage errors
- **WHEN** disk is full or write fails
- **THEN** error is propagated to caller
