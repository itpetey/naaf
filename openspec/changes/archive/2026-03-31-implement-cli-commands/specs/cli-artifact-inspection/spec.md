## ADDED Requirements

### Requirement: artifacts command lists run artifacts
The CLI SHALL list all artifacts for a given run.

#### Scenario: List artifacts for run
- **GIVEN** a run with multiple artifacts
- **WHEN** `naaf artifacts <run-id>` is executed
- **THEN** all artifacts are listed with ID, kind, and timestamp

#### Scenario: Run has no artifacts
- **GIVEN** a run with no artifacts
- **WHEN** artifacts command is executed
- **THEN** "No artifacts found" is displayed

#### Scenario: Run does not exist
- **GIVEN** an invalid run ID
- **WHEN** artifacts command is executed
- **THEN** an error message is shown

### Requirement: artifacts command supports viewing content
The CLI SHALL allow viewing individual artifact content.

#### Scenario: View artifact with --view flag
- **GIVEN** an artifact ID
- **WHEN** `naaf artifacts <run-id> --view <artifact-id>` is executed
- **THEN** the artifact content is displayed

### Requirement: artifacts command supports JSON output
The CLI SHALL support JSON format for scripting.

#### Scenario: JSON output
- **WHEN** `naaf artifacts <run-id> --json` is executed
- **THEN** artifacts are output as JSON
