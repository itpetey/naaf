## ADDED Requirements

### Requirement: TryFromState trait
The system SHALL provide a TryFromState trait with method `fn try_from_state(state: &StateEnvelope) -> Result<Self, StepError>` where Self: Sized.

#### Scenario: Typed extraction
- **WHEN** calling try_from_state on a type
- **THEN** it returns the typed value or error
