## ADDED Requirements

### Requirement: run_workflow executes happy path end-to-end
The system SHALL execute all transitions in the happy-path workflow.

#### Scenario: Full workflow execution
- **GIVEN** a run with UserPrompt artifact
- **AND** the happy-path workflow
- **WHEN** run_workflow() is called
- **THEN** all 4 transitions execute in order
- **AND** 4 artifacts are produced

### Requirement: run_workflow stops at terminal phase
The system SHALL stop execution when reaching a terminal phase.

#### Scenario: Reached terminal phase
- **GIVEN** a run at Phase::Accepted (terminal)
- **WHEN** run_workflow() is called
- **THEN** no more transitions are attempted
- **AND** Outcome::Done is returned

### Requirement: run_workflow returns outcome
The system SHALL return the final outcome of the workflow.

#### Scenario: Successful completion
- **GIVEN** all transitions succeed
- **WHEN** run_workflow() completes
- **THEN** Outcome::Done is returned

#### Scenario: Transition fails
- **GIVEN** a transition fails
- **WHEN** run_workflow() returns
- **THEN** Outcome::Failed is returned with reason

### Requirement: run_workflow produces artifacts at each step
The system SHALL persist artifacts after each transition.

#### Scenario: After first transition
- **GIVEN** workflow has executed first transition
- **WHEN** artifacts are queried
- **THEN** NormalizedSpec artifact exists

#### Scenario: After all transitions
- **GIVEN** workflow completes
- **WHEN** artifacts are queried
- **THEN** all 4 artifacts exist: NormalizedSpec, ScopeReport, ProposalSkeleton, AcceptanceCriteriaSet
