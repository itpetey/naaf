## ADDED Requirements

### Requirement: workflow-schema crate exists
The system SHALL include a `workflow-schema` crate providing shared runtime state and structured artifacts including `StateEnvelope`, `StateKind`, `ArtifactKey`, artifact structs, validation contracts, typed accessors, and workflow input/output contracts.

#### Scenario: Crate compiles
- **WHEN** running `cargo build -p workflow-schema`
- **THEN** the crate compiles without errors

### Requirement: workflow-schema has module structure
The system SHALL provide the following modules: `state.rs`, `artifacts.rs`, `contracts.rs`, `meta.rs`, `lineage.rs`, `validation.rs`.

#### Scenario: Module structure exists
- **WHEN** examining `workflow-schema/src/`
- **THEN** all specified modules are present as empty modules
