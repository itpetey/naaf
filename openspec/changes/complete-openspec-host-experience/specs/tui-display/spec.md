## MODIFIED Requirements

### Requirement: TUI displays current state
The system SHALL provide an interactive TUI that displays workflow catalogue information, execution state, and saved run inspection details.

#### Scenario: Workflow catalogue view
- **WHEN** a user enters the TUI host
- **THEN** the TUI shows available workflows and allows workflow selection before execution starts

#### Scenario: Execution state view
- **WHEN** a workflow is executing in the TUI
- **THEN** the TUI displays the current step, state kind, and key artifacts for the active run

#### Scenario: Saved run inspection view
- **WHEN** a user inspects a saved run in the TUI
- **THEN** the TUI displays the execution trace, final state summary, and workflow-specific primary outputs
