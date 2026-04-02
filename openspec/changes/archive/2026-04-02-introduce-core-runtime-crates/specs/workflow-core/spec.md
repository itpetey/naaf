## ADDED Requirements

### Requirement: workflow-core crate exists
The system SHALL include a `workflow-core` crate providing runtime concerns including workflow builder DSL, workflow compilation, execution engine, step traits, routing model, fork/join semantics, budgets/limits/cancellation, execution events and tracing, persistence interfaces, and shared runtime errors.

#### Scenario: Crate compiles
- **WHEN** running `cargo build -p workflow-core`
- **THEN** the crate compiles without errors

### Requirement: workflow-core has module structure
The system SHALL provide the following modules: `builder.rs`, `compiled.rs`, `executor.rs`, `graph.rs`, `steps.rs`, `route.rs`, `join.rs`, `budget.rs`, `events.rs`, `errors.rs`.

#### Scenario: Module structure exists
- **WHEN** examining `workflow-core/src/`
- **THEN** all specified modules are present as empty modules
