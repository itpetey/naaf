## ADDED Requirements

### Requirement: TUI backlog document exists
The system SHALL have a documented backlog for future TUI work.

#### Scenario: Backlog document created
- **WHEN** the backlog is created
- **THEN** it contains feature name, priority, description for each item

### Requirement: Backlog includes run supervision features
The backlog SHALL include features for monitoring runs.

#### Scenario: Supervision features listed
- **GIVEN** the TUI backlog
- **WHEN** it is inspected
- **THEN** it includes: run status dashboard, artifact viewer, event timeline

### Requirement: Backlog includes run control features
The backlog SHALL include features for controlling runs.

#### Scenario: Control features listed
- **GIVEN** the TUI backlog
- **WHEN** it is inspected
- **THEN** it includes: resume run, abort run, retry transition

### Requirement: TUI is explicitly not implemented
**This requirement is now stale.** The TUI exists as the primary workflow host.

**Migration**: Consider archiving or updating this spec to reflect current state.
