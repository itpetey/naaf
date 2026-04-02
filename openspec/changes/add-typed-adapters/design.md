## Context

The runtime stays envelope-based, but steps benefit from typed access to artifacts.

## Goals / Non-Goals

**Goals:**
- Define TryFromState and IntoState traits
- Implement common adapters

**Non-Goals:**
- Change runtime to use typed states

## Decisions

1. **Trait design**
   - Decision: Return Result for TryFromState
   - Rationale: Clear error handling

## Risks / Trade-offs

- [Low] Adapter patterns well-known → Low risk
