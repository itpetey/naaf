## Context

The first workflow must handle greeting ("Hi"), ambiguous ("Help me improve this"), and actionable requests. These built-ins enable that.

## Goals / Non-Goals

**Goals:**
- Implement reusable classification routers
- Handle all three input classes

**Non-Goals:**
- Implement specific prompts (that's later)

## Decisions

1. **Classification approach**
   - Decision: Use confidence scoring
   - Rationale: Simple, effective for v1

## Risks / Trade-offs

- [Low] Well-known patterns → Low risk
