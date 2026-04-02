## ADDED Requirements

### Requirement: workflow-llm crate exists
The system SHALL include a `workflow-llm` crate providing model invocation concerns including prompt rendering, structured output parsing, retry/repair loop helpers, provider-independent LLM execution helpers, and token/cost accounting.

#### Scenario: Crate compiles
- **WHEN** running `cargo build -p workflow-llm`
- **THEN** the crate compiles without errors

### Requirement: workflow-llm has module structure
The system SHALL provide the following modules: `client.rs`, `prompt.rs`, `structured_output.rs`, `repair.rs`, `usage.rs`.

#### Scenario: Module structure exists
- **WHEN** examining `workflow-llm/src/`
- **THEN** all specified modules are present as empty modules
