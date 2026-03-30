## ADDED Requirements

### Requirement: run command executes workflow
The CLI SHALL execute the happy-path workflow when run command is invoked.

#### Scenario: Run with prompt argument
- **GIVEN** the CLI and a prompt string
- **WHEN** `naaf run "add user authentication"` is executed
- **THEN** the workflow executes from UserPrompt through all phases
- **AND** artifacts are persisted

#### Scenario: Run completes successfully
- **GIVEN** a successful workflow execution
- **WHEN** the run command completes
- **THEN** output shows "Completed successfully"
- **AND** output shows the run ID and artifact location

#### Scenario: Run fails
- **GIVEN** a workflow that fails
- **WHEN** the run command completes
- **THEN** output shows "Failed: <reason>"
- **AND** output shows the run ID for inspection

### Requirement: run command validates API key
The CLI SHALL check for API key before executing.

#### Scenario: Missing API key
- **GIVEN** OPENAI_API_KEY is not set
- **WHEN** run command is invoked
- **THEN** an error message is shown with setup instructions

### Requirement: run command shows progress
The CLI SHALL show progress during execution.

#### Scenario: Execution in progress
- **WHEN** workflow is executing
- **THEN** progress messages are displayed
