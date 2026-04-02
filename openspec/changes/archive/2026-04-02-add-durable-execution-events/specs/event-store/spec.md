## ADDED Requirements

### Requirement: FilesystemEventStore
The system SHALL provide a FilesystemEventStore that persists events to filesystem.

#### Scenario: Event persistence
- **WHEN** writing events to FilesystemEventStore
- **THEN** events are stored persistently
