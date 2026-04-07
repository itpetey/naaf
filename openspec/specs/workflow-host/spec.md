## ADDED Requirements

### Requirement: Host lists discovered workflows
The system SHALL provide a workflow host surface that lists discovered workflow packages with their user-facing metadata.

#### Scenario: Show workflow catalogue
- **WHEN** the host starts and discovers available workflow packages
- **THEN** it displays each workflow's identifier, name, and summary for selection

### Requirement: Host runs a selected workflow package
The system SHALL execute a selected workflow package through the generic workflow executor.

#### Scenario: Run selected workflow package
- **WHEN** a user selects a discovered workflow package and provides the required input
- **THEN** the host builds the executable workflow from the package manifest and runs it

#### Scenario: Workflow package load failure
- **WHEN** the selected workflow package cannot be built from its manifest and registered steps
- **THEN** the host reports the load failure and does not start execution

### Requirement: Host inspects saved runs
The system SHALL allow users to inspect persisted workflow runs from the workflow host surface.

#### Scenario: Inspect saved run
- **WHEN** a user selects a saved run for inspection
- **THEN** the host displays the run's recorded execution details and final state summary

### Requirement: Host replays saved runs
The system SHALL allow users to replay a saved workflow run from the workflow host surface.

#### Scenario: Replay saved run
- **WHEN** a user chooses to replay a saved run
- **THEN** the host starts a new execution using the workflow and input associated with the saved run
