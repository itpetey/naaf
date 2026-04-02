## ADDED Requirements

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
