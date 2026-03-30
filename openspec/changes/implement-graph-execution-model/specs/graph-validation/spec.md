## ADDED Requirements

### Requirement: GraphWorkflow validates presence of entry node
GraphWorkflow SHALL fail validation if no entry node exists.

#### Scenario: Missing entry node
- **GIVEN** a WorkflowDefinition with no phases
- **WHEN** GraphWorkflow::from_workflow() is called with validation
- **THEN** a ValidationError::MissingEntryNode is returned

### Requirement: GraphWorkflow validates all nodes are reachable
GraphWorkflow SHALL detect unreachable nodes (nodes not reachable from entry).

#### Scenario: Unreachable node
- **GIVEN** a WorkflowDefinition with phases A, B, C where A -> B but C is isolated
- **WHEN** GraphWorkflow::from_workflow() is called with validation
- **THEN** a ValidationError::UnreachableNode(Phase) is returned

### Requirement: GraphWorkflow validates terminal phase configuration
GraphWorkflow SHALL ensure terminal phases are properly defined.

#### Scenario: No terminal phases
- **GIVEN** a WorkflowDefinition where every phase has outgoing transitions (cycle)
- **WHEN** GraphWorkflow::from_workflow() is called with validation
- **THEN** a ValidationError::NoTerminalPhase is returned

### Requirement: GraphWorkflow validates transition references valid phases
GraphWorkflow SHALL fail if transitions reference non-existent phases.

#### Scenario: Invalid transition
- **GIVEN** a WorkflowDefinition with a transition from Phase A to Phase X (which doesn't exist)
- **WHEN** GraphWorkflow::from_workflow() is called with validation
- **THEN** a ValidationError::InvalidTransition is returned

### Requirement: Validation returns all errors
GraphWorkflow validation SHALL collect all errors, not fail on first.

#### Scenario: Multiple validation errors
- **GIVEN** a WorkflowDefinition with multiple validation issues
- **WHEN** validation is performed
- **THEN** a Vec of all ValidationError values is returned

### Requirement: ValidationError provides human-readable messages
ValidationError SHALL implement Display for error reporting.

#### Scenario: Error message format
- **GIVEN** a ValidationError::UnreachableNode(Phase::Proposed)
- **WHEN** the error is formatted as a string
- **THEN** the message includes "Proposed" and describes the issue
