## Context

Every step transition must emit an event for durable tracing. This enables debugging, replay, and inspection.

## Goals / Non-Goals

**Goals:**
- Define event schema with all required event types
- Create TraceSink trait for event emission
- Implement filesystem-based event store

**Non-Goals:**
- Implement database backend (filesystem is sufficient for v1)
- Implement replay logic (that's later)

## Decisions

1. **Event format**
   - Decision: Use JSON lines format for event log
   - Rationale: Simple, append-only, easy to parse

2. **TraceSink**
   - Decision: Use trait with sync/async variants
   - Rationale: Flexibility for different backends

## Risks / Trade-offs

- [Low] Event schema may evolve → Acceptable
