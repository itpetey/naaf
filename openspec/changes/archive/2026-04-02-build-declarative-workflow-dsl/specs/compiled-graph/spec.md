## ADDED Requirements

### Requirement: CompiledWorkflow structure
The system SHALL provide a CompiledWorkflow struct containing nodes, edges, and entry point.

#### Scenario: Graph structure
- **WHEN** examining compiled workflow
- **THEN** it contains valid nodes and edges

### Requirement: Graph is acyclic
The system SHALL validate that the compiled graph is acyclic (v1).

#### Scenario: Cyclic graph
- **WHEN** compiling a workflow with cycles
- **THEN** compilation fails with appropriate error
