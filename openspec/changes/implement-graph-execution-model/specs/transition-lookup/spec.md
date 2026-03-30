## ADDED Requirements

### Requirement: Transition lookup returns executable transitions from current phase
GraphWorkflow SHALL return all transitions executable from a given run phase.

#### Scenario: Single outgoing transition
- **GIVEN** a GraphWorkflow with A -> B transition
- **WHEN** executable_transitions(Phase::A) is called
- **THEN** a vector containing the A->B TransitionSpec is returned

#### Scenario: Multiple outgoing transitions
- **GIVEN** a GraphWorkflow with A -> B and A -> C transitions
- **WHEN** executable_transitions(Phase::A) is called
- **THEN** both transitions are returned

#### Scenario: No outgoing transitions from terminal phase
- **GIVEN** a GraphWorkflow where Phase::Completed is terminal
- **WHEN** executable_transitions(Phase::Completed) is called
- **THEN** an empty vector is returned

### Requirement: Transition lookup filters by required artifacts
Executable transitions SHALL only be returned if required artifacts are available.

#### Scenario: Required artifact available
- **GIVEN** a GraphWorkflow with transition A -> B that requires ArtifactKind::UserPrompt
- **AND** the available artifacts include UserPrompt
- **WHEN** executable_transitions(Phase::A, &[ArtifactKind::UserPrompt]) is called
- **THEN** the transition is returned

#### Scenario: Required artifact missing
- **GIVEN** a GraphWorkflow with transition A -> B that requires ArtifactKind::UserPrompt
- **AND** the available artifacts do NOT include UserPrompt
- **WHEN** executable_transitions(Phase::A, &[]) is called
- **THEN** the transition is NOT returned

### Requirement: Transition lookup handles unknown phases gracefully
Lookup for a phase not in the graph SHALL return empty vector.

#### Scenario: Unknown phase
- **GIVEN** a GraphWorkflow
- **WHEN** executable_transitions(UnknownPhase) is called
- **THEN** an empty vector is returned (no error)

### Requirement: Transition lookup supports optional artifact filtering
The artifact filter parameter SHALL be optional for backward compatibility.

#### Scenario: No artifact filter provided
- **GIVEN** a GraphWorkflow with transition A -> B
- **WHEN** executable_transitions(Phase::A) is called (no artifact filter)
- **THEN** all outgoing transitions are returned regardless of artifact requirements
