## ADDED Requirements

### Requirement: journal command displays events
The CLI SHALL display journal events for a given run.

#### Scenario: Display journal for run
- **GIVEN** a run with journal events
- **WHEN** `naaf journal <run-id>` is executed
- **THEN** events are displayed in chronological order
- **AND** each line shows timestamp, event type, and details

#### Scenario: Run has no journal
- **GIVEN** a run with no journal
- **WHEN** journal command is executed
- **THEN** "No journal entries found" is displayed

#### Scenario: Journal is empty
- **GIVEN** a run that never started
- **WHEN** journal command is executed
- **THEN** appropriate message is shown

### Requirement: journal command supports filtering
The CLI SHALL allow filtering events by type.

#### Scenario: Filter by event type
- **WHEN** `naaf journal <run-id> --filter TransitionExecuted` is executed
- **THEN** only TransitionExecuted events are shown
