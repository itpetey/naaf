## ADDED Requirements

### Requirement: Run workflow command
The system SHALL provide a CLI command to run a workflow with input.

#### Scenario: Run command
- **WHEN** running `naaf run <workflow> <input>`
- **THEN** workflow executes and returns result

### Requirement: Show trace command
The system SHALL provide a CLI command to show run trace.

#### Scenario: Trace command
- **WHEN** running `naaf trace <run-id>`
- **THEN** trace is displayed
