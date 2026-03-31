## ADDED Requirements

### Requirement: WorkerExecutor renders prompt from worker spec
The system SHALL provide a WorkerExecutor that renders prompts from worker specs and input artifacts.

#### Scenario: Render prompt with single artifact
- **GIVEN** a WorkerSpec and one input artifact content
- **WHEN** WorkerExecutor.render_prompt() is called
- **THEN** the prompt template has artifact content substituted for variables

#### Scenario: Render prompt with multiple artifacts
- **GIVEN** a WorkerSpec requiring two artifacts
- **WHEN** WorkerExecutor.render_prompt() is called
- **THEN** both artifacts are included in the rendered prompt

#### Scenario: Render includes system prompt
- **WHEN** a prompt is rendered
- **THEN** it SHALL include a system message directing the model behavior

### Requirement: WorkerExecutor calls ModelProvider
The system SHALL send rendered prompts to the ModelProvider.

#### Scenario: Successful model call
- **GIVEN** a rendered prompt
- **WHEN** WorkerExecutor.execute() is called
- **THEN** ModelProvider.generate() is called
- **AND** the response content is returned

### Requirement: WorkerExecutor handles model errors
The system SHALL propagate provider errors as EngineError.

#### Scenario: Provider returns error
- **GIVEN** ModelProvider returns ProviderError::RateLimited
- **WHEN** WorkerExecutor.execute() is called
- **THEN** EngineError::WorkerFailed is returned with details
