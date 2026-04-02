## ADDED Requirements

### Requirement: WorkflowBuilder API
The system SHALL provide a WorkflowBuilder that supports methods: `new(name)`, `step(id, transformer)`, `route(id, router)`, `branch(id)`, `path(id, step)`, `join(id, reducer)`, `step(id, transformer)`, `terminal(id)`, `compile()`.

#### Scenario: Builder creates workflow
- **WHEN** calling WorkflowBuilder methods to define a workflow
- **THEN** the builder accumulates the definition

### Requirement: Builder returns CompiledWorkflow
The system SHALL ensure `compile()` returns a `CompiledWorkflow` or error.

#### Scenario: Compile validation
- **WHEN** calling compile() on invalid workflow
- **THEN** it returns a validation error
