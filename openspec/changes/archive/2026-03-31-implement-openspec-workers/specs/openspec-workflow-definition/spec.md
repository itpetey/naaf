## ADDED Requirements

### Requirement: OpenSpec happy-path workflow exists
The system SHALL provide a WorkflowDefinition for the OpenSpec happy path.

#### Scenario: Create happy-path workflow
- **WHEN** openspec_happy_path() is called
- **THEN** a WorkflowDefinition is returned with 4 phases and 4 transitions

### Requirement: Workflow has correct entry phase
The happy-path workflow SHALL start at Phase::Proposed.

#### Scenario: Query entry phase
- **GIVEN** the happy-path workflow
- **WHEN** entry_phase() is called
- **THEN** Phase::Proposed is returned

### Requirement: First transition is RequestNormalizer
The first transition SHALL transform UserPrompt to NormalizedSpec.

#### Scenario: First transition details
- **GIVEN** the happy-path workflow
- **WHEN** outgoing_transitions(Phase::Proposed) is called
- **THEN** one transition is returned with to_phase: Phase::Normalized and produces: ArtifactKind::NormalizedSpec

### Requirement: Second transition is ScopeAnalyst
The second transition SHALL transform NormalizedSpec to ScopeReport.

#### Scenario: Second transition details
- **GIVEN** the happy-path workflow
- **WHEN** outgoing_transitions(Phase::Normalized) is called
- **THEN** one transition is returned with to_phase: Phase::Scoped and produces: ArtifactKind::ScopeReport

### Requirement: Third transition is ProposalSkeletonBuilder
The third transition SHALL combine NormalizedSpec and ScopeReport into ProposalSkeleton.

#### Scenario: Third transition details
- **GIVEN** the happy-path workflow
- **WHEN** outgoing_transitions(Phase::Scoped) is called
- **THEN** one transition is returned with to_phase: Phase::Planned and produces: ArtifactKind::ProposalSkeleton

### Requirement: Fourth transition is AcceptanceCriteriaAuthor
The fourth transition SHALL transform ProposalSkeleton + NormalizedSpec into AcceptanceCriteriaSet.

#### Scenario: Fourth transition details
- **GIVEN** the happy-path workflow
- **WHEN** outgoing_transitions(Phase::Planned) is called
- **THEN** one transition is returned with to_phase: Phase::Accepted and produces: ArtifactKind::AcceptanceCriteriaSet

### Requirement: Phase::Accepted is terminal
The final phase SHALL be terminal (no outgoing transitions).

#### Scenario: Terminal phase check
- **GIVEN** the happy-path workflow
- **WHEN** is_terminal_phase(Phase::Accepted) is called
- **THEN** true is returned

### Requirement: Workflow validates successfully
The happy-path workflow SHALL pass graph validation.

#### Scenario: Validation passes
- **GIVEN** the happy-path workflow
- **WHEN** it is validated
- **THEN** no validation errors are returned
