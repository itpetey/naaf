## MODIFIED Requirements

### Requirement: Run workflow command
The system SHALL provide a CLI command to run a workflow with input.

#### Scenario: Run command
- **WHEN** running `naaf run <workflow> <input>`
- **THEN** workflow executes and returns result

#### Scenario: Ambiguous run in interactive terminal
- **WHEN** `naaf run <workflow> <input>` completes with an ambiguous escalation in an interactive terminal
- **THEN** the CLI prompts for one clarification
- **AND** the CLI starts a new run using the original input and the clarification

#### Scenario: Ambiguous run in non-interactive mode
- **WHEN** `naaf run <workflow> <input>` completes with an ambiguous escalation while stdin or stdout is not a terminal
- **THEN** the CLI prints the escalation details
- **AND** the CLI exits without prompting for clarification

#### Scenario: Clarification still ambiguous
- **WHEN** the follow-up run also completes with an ambiguous escalation
- **THEN** the CLI prints the escalation details
- **AND** the CLI does not prompt a second time
