## ADDED Requirements

### Requirement: Validator trait
The system SHALL provide a `Validator` trait with method `name() -> &'static str` and `validate(&self, ctx: &ExecCtx, state: &StateEnvelope) -> Result<(), ValidationError>`.

#### Scenario: Validation execution
- **WHEN** calling validate on a Validator
- **THEN** it returns Ok(()) if valid or ValidationError if invalid

### Requirement: ValidationError type
The system SHALL provide a ValidationError type that includes the validator name and reason.

#### Scenario: Validation failure
- **WHEN** Validator fails
- **THEN** error includes which validator failed and why
