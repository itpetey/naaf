## ADDED Requirements

### Requirement: Router trait
The system SHALL provide a `Router` trait with method `name() -> &'static str` and `route(&self, ctx: &mut ExecCtx, state: &StateEnvelope) -> Result<RouteDecision, StepError>`.

#### Scenario: Router execution
- **WHEN** calling route on a Router
- **THEN** it returns a RouteDecision or StepError

### Requirement: RouteDecision type
The system SHALL provide a RouteDecision enum with variants for single next step, branch paths, or terminal completion.

#### Scenario: Route decision
- **WHEN** Router returns RouteDecision
- **THEN** executor knows which step(s) to execute next
