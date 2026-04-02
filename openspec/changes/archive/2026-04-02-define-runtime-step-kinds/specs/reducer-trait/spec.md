## ADDED Requirements

### Requirement: Reducer trait
The system SHALL provide a `Reducer` trait with method `name() -> &'static str` and `reduce(&self, ctx: &mut ExecCtx, inputs: Vec<StateEnvelope>) -> Result<StateEnvelope, StepError>`.

#### Scenario: Reducer execution
- **WHEN** calling reduce on a Reducer with multiple inputs
- **THEN** it returns a merged StateEnvelope or StepError

### Requirement: Reducer handles empty inputs
The system SHALL define reducer behavior for empty input vector.

#### Scenario: Empty inputs
- **WHEN** Reducer receives empty inputs
- **THEN** it returns an appropriate error
