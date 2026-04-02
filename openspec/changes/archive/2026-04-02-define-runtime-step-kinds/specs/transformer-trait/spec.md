## ADDED Requirements

### Requirement: Transformer trait
The system SHALL provide a `Transformer` trait with method `name() -> &'static str` and `transform(&self, ctx: &mut ExecCtx, input: StateEnvelope) -> Result<StateEnvelope, StepError>`.

#### Scenario: Transformer execution
- **WHEN** calling transform on a Transformer
- **THEN** it returns a new StateEnvelope or StepError

### Requirement: Transformer is Send + Sync
The system SHALL ensure Transformer implementations are Send + Sync for thread safety.

#### Scenario: Thread safety
- **WHEN** moving a Transformer across thread boundaries
- **THEN** it compiles without lifetime issues
