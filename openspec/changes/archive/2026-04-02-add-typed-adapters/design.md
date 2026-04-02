## Context

The runtime stays envelope-based, but steps benefit from typed access to artifacts.

## Goals / Non-Goals

**Goals:**
- Define TryFromState and IntoState traits
- Implement common adapters
- Support multiple artifacts with named keys

**Non-Goals:**
- Change runtime to use typed states

## Decisions

1. **Trait design**
   - Decision: Return Result for TryFromState
   - Rationale: Clear error handling

2. **Error type location**
   - Decision: Use AdapterError in workflow-schema, not StepError
   - Rationale: StepError is in workflow-core which depends on workflow-schema, creating a circular dependency if reversed. AdapterError stays in schema layer where traits are defined.
   - Integration: workflow-core implements From<AdapterError> for StepError

3. **Artifact key parameterization**
   - Decision: All trait methods take ArtifactKey parameter
   - Rationale: Real workflows need multiple artifacts per state (input, output, metadata, etc.)
   - Hard-coded "value" key would limit utility

## Risks / Trade-offs

- [Low] Adapter patterns well-known → Low risk
- [Medium] Error type split across crates → Requires conversion impl
