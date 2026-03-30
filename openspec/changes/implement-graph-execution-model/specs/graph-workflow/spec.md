## ADDED Requirements

### Requirement: GraphWorkflow can be constructed from WorkflowDefinition
GraphWorkflow SHALL be constructed from an existing WorkflowDefinition using a from_workflow() method.

#### Scenario: Construction from valid workflow
- **GIVEN** a valid WorkflowDefinition with phases and transitions
- **WHEN** GraphWorkflow::from_workflow() is called
- **THEN** a GraphWorkflow instance is returned
- **AND** the internal petgraph contains nodes for each Phase
- **AND** edges represent transitions between phases

#### Scenario: Empty workflow
- **GIVEN** a WorkflowDefinition with no phases
- **WHEN** GraphWorkflow::from_workflow() is called
- **THEN** an empty graph is created

### Requirement: GraphWorkflow provides entry node query
GraphWorkflow SHALL provide a method to retrieve the entry phase/node.

#### Scenario: Query entry node
- **GIVEN** a GraphWorkflow built from a workflow
- **WHEN** entry_phase() is called
- **THEN** the Phase of the entry node is returned

### Requirement: GraphWorkflow provides terminal nodes query
GraphWorkflow SHALL provide a method to retrieve all terminal phases (nodes with no outgoing edges).

#### Scenario: Query terminal nodes
- **GIVEN** a GraphWorkflow with multiple terminal phases
- **WHEN** terminal_phases() is called
- **THEN** a vector of all terminal Phase values is returned

#### Scenario: Single path workflow
- **GIVEN** a GraphWorkflow representing a linear path A -> B -> C
- **WHEN** terminal_phases() is called
- **THEN** only phase C is returned

### Requirement: GraphWorkflow provides phase to node index mapping
GraphWorkflow SHALL map Phase values to internal graph node indices.

#### Scenario: Phase to index mapping
- **GIVEN** a GraphWorkflow
- **WHEN** node_index(phase) is called with a valid phase
- **THEN** the corresponding petgraph node index is returned

#### Scenario: Unknown phase mapping
- **GIVEN** a GraphWorkflow
- **WHEN** node_index(unknown_phase) is called
- **THEN** None is returned
