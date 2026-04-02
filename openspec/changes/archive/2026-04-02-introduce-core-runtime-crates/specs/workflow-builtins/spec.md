## ADDED Requirements

### Requirement: workflow-builtins crate exists
The system SHALL include a `workflow-builtins` crate providing reusable step implementations including classify ambiguity, normalize input, validate schema, branch by confidence, reduce parallel outputs, accept/reject gates, terminal handlers, and escalation handlers.

#### Scenario: Crate compiles
- **WHEN** running `cargo build -p workflow-builtins`
- **THEN** the crate compiles without errors

### Requirement: workflow-builtins has module structure
The system SHALL provide the following modules: `classify_input.rs`, `normalize.rs`, `clarify.rs`, `accept.rs`, `reducers.rs`, `validators.rs`, `terminal.rs`.

#### Scenario: Module structure exists
- **WHEN** examining `workflow-builtins/src/`
- **THEN** all specified modules are present as empty modules
