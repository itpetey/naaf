## ADDED Requirements

### Requirement: Packaged OpenSpec authoring workflow
The system SHALL provide a packaged OpenSpec workflow that supports LLM-backed proposal drafting and acceptance.

#### Scenario: Start OpenSpec authoring run
- **WHEN** a user selects the packaged OpenSpec authoring workflow and provides the required runtime configuration
- **THEN** the host executes the LLM-backed OpenSpec workflow through the packaged workflow path

### Requirement: OpenSpec workflow includes review and remediation stages
The packaged OpenSpec authoring workflow SHALL include review and remediation stages in addition to the initial drafting path.

#### Scenario: Review identifies issues
- **WHEN** review stages find issues in the proposal
- **THEN** the workflow records those findings and enters remediation rather than terminating at the initial draft

#### Scenario: Workflow reaches acceptance after remediation
- **WHEN** remediation resolves the outstanding issues
- **THEN** the workflow proceeds to acceptance and records an accepted OpenSpec outcome

### Requirement: OpenSpec workflow surfaces clarification and escalation outcomes
The packaged OpenSpec authoring workflow SHALL preserve ambiguity, clarification, and escalation behaviour in the host experience.

#### Scenario: Clarification required
- **WHEN** the OpenSpec workflow cannot proceed safely from the provided request
- **THEN** the host shows the clarification or escalation outcome produced by the workflow package
