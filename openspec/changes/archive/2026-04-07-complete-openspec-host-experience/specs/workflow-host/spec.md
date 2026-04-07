## ADDED Requirements

### Requirement: Host collects workflow runtime configuration
The system SHALL collect and validate the runtime configuration required by a selected workflow package before execution begins.

#### Scenario: Configure provider-backed workflow
- **WHEN** a selected workflow package declares provider and model requirements
- **THEN** the host prompts for those values or resolves them from configured host defaults before execution

#### Scenario: Reject incomplete workflow configuration
- **WHEN** required workflow configuration is missing or invalid
- **THEN** the host blocks execution and reports which configuration is still required

### Requirement: Host presents workflow catalogue and detail views
The system SHALL provide interactive workflow catalogue and workflow detail flows instead of only command-style execution entry points.

#### Scenario: Browse workflow catalogue
- **WHEN** a user enters the workflow host
- **THEN** the host shows the available workflows and allows the user to inspect one before running it

#### Scenario: Inspect workflow details before run
- **WHEN** a user selects a workflow from the catalogue
- **THEN** the host shows the workflow summary, runtime requirements, and expected outputs before execution starts

### Requirement: Host inspects saved runs with workflow-aware output
The system SHALL inspect saved runs using workflow-aware metadata rather than only raw event output.

#### Scenario: Inspect saved workflow run
- **WHEN** a user opens a saved run
- **THEN** the host shows workflow identity, execution trace, final state, and package-declared primary outputs
