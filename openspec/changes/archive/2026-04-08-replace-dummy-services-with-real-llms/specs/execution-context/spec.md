## MODIFIED Requirements

### Requirement: ExecCtx provides runtime context
The system SHALL provide an ExecCtx with fields: run_id, budget, services, trace, cancel.

#### Scenario: Context access
- **WHEN** a step accesses ExecCtx
- **THEN** it has access to run ID, budget state, services, trace sink, cancellation token

### Requirement: Services trait
The system SHALL define a Services trait for runtime services (LLM clients, etc.).

#### Scenario: Service injection
- **WHEN** creating Executor
- **THEN** services can be injected for testing and different backends

### Requirement: ExecCtx supports real LLM service
The ExecCtx SHALL work with either `DummyServices` for testing or a real `LlmService` for production.

#### Scenario: Real LLM service in production
- **WHEN** an `ExecCtx<LlmService>` is created with a configured provider
- **THEN** the context can make actual LLM calls via the services field

#### Scenario: Dummy services for testing
- **WHEN** an `ExecCtx<DummyServices>` is created
- **THEN** the context works without any real LLM configuration (for deterministic test responses)