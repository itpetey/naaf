## MODIFIED Requirements

### Requirement: Executor runs compiled workflow
The system SHALL provide an Executor that executes a CompiledWorkflow from an initial StateEnvelope.

#### Scenario: Execute workflow
- **WHEN** calling Executor.execute() with compiled workflow and initial state
- **THEN** it returns a final StateEnvelope or error

### Requirement: Executor invokes steps
The system SHALL ensure the executor calls the appropriate step methods based on node type.

#### Scenario: Step invocation
- **WHEN** executing a step node
- **THEN** the executor calls the step's method (transform/route/reduce/validate)

### Requirement: Executor uses configurable services
The Executor SHALL accept a configurable services implementation rather than hardcoded `DummyServices`.

#### Scenario: Executor with LlmService
- **WHEN** Executor is created with `LlmService::from_config(config)`
- **THEN** workflow steps can make real LLM calls during execution

#### Scenario: Executor with DummyServices for tests
- **WHEN** Executor is created with `DummyServices::default()`
- **THEN** workflow execution uses mock responses for deterministic testing