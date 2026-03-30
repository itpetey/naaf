## ADDED Requirements

### Requirement: ExecutionEngine accepts ModelProvider
The system SHALL allow ExecutionEngine to use a ModelProvider for worker execution.

#### Scenario: Create ExecutionEngine with provider
- **GIVEN** a ModelProvider implementation
- **WHEN** ExecutionEngine is constructed
- **THEN** it SHALL store the provider for later use

### Requirement: Orchestrator does not depend on provider-openai
The orchestrator crate SHALL only depend on the model crate, not provider-openai.

#### Scenario: Check Cargo.toml dependencies
- **GIVEN** the orchestrator crate
- **WHEN** Cargo.toml is inspected
- **THEN** provider-openai is NOT listed as a dependency

### Requirement: Worker execution uses ModelProvider
The system SHALL call the ModelProvider when executing workers that need LLM inference.

#### Scenario: Execute worker with LLM call
- **GIVEN** a worker that requires LLM inference
- **WHEN** the transition is executed
- **THEN** the ModelProvider.generate() method is called
- **AND** the response is used to produce the output artifact

### Requirement: Provider configuration is external to orchestrator
Provider configuration SHALL be handled outside the orchestrator, allowing runtime provider selection.

#### Scenario: Inject provider at runtime
- **GIVEN** an ExecutionEngine
- **WHEN** it is created with a specific ModelProvider
- **THEN** the orchestrator has no knowledge of how the provider was configured
