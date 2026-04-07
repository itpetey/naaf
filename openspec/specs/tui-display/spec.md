## ADDED Requirements

### Requirement: TUI displays current state
The system SHALL provide a workflow-aware TUI that displays available workflows, the current step, state kind, and key artifacts during workflow execution.

#### Scenario: Show workflow catalogue
- **WHEN** running the TUI
- **THEN** the TUI displays the available workflows for selection before execution starts

#### Scenario: TUI display during workflow execution
- **WHEN** a workflow is executing in the TUI
- **THEN** the current step, state kind, and key artifacts are displayed

### Requirement: TUI inspects saved runs
The system SHALL provide a TUI flow for inspecting saved workflow runs.

#### Scenario: Inspect run in TUI
- **WHEN** a user chooses a saved run from the TUI
- **THEN** the TUI displays the run's execution events and final state summary
