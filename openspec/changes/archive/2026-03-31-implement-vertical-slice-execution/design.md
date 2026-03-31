## Context

This phase wires together all previous work to execute the OpenSpec workflow end-to-end. We have:
- Orchestrator with ExecutionEngine skeleton
- Model provider abstraction (Phase 4)
- OpenSpec workers with prompt templates (Phase 5)
- Artifact stores and journal (Phase 2)

Now we need to connect these pieces into a working execution path.

## Goals / Non-Goals

**Goals:**
- Implement prompt rendering from worker spec + input artifacts
- Implement LLM response parsing into artifact structs
- Execute single transition end-to-end with persistence
- Execute full happy-path workflow end-to-end

**Non-Goals:**
- Review or remediation loops (Phase 8-9)
- Error recovery beyond basic retry
- Complex state management

## Decisions

### Decision 1: Prompt Rendering

Create a WorkerExecutor in orchestrator that:
1. Takes WorkerSpec + input artifacts
2. Renders prompt template with artifact content as variables
3. Sends to ModelProvider
4. Returns raw response

Rationale: Keeps prompt rendering in orchestrator, specific decoding in openspec.

### Decision 2: Output Decoding

Create decode functions in openspec crate:
- `decode_normalized_spec(text: &str) -> Result<NormalizedSpec, DecodeError>`
- `decode_scope_report(text: &str) -> Result<ScopeReport, DecodeError>`
- etc.

Rationale: Each artifact type knows how to parse itself. Uses JSON for structured output.

### Decision 3: Transition Execution Flow

ExecuteTransition takes:
- Run context
- TransitionSpec
- Input artifacts

Steps:
1. Load required artifacts from store
2. Render prompt with WorkerExecutor
3. Call ModelProvider
4. Decode output with openspec decoder
5. Save new artifact to store
6. Update run phase
7. Record journal event

Rationale: Simple linear flow for v1.

### Decision 4: Workflow Runner

Create run_workflow function that:
1. Takes initial UserPrompt artifact
2. Loops through transitions until terminal
3. Returns final outcome

Rationale: Simple loop. Each transition advances the phase.

### Decision 5: Handling Decode Errors

If LLM output cannot be parsed:
- Log the failure
- Return EngineError::ParseError
- Orchestrator handles via retry limit

Rationale: Fail fast rather than guess.

## Risks / Trade-offs

- [Risk] LLM output format inconsistency → [Mitigation] Use strict JSON schema in prompts; retry on parse failure
- [Risk] Long-running workflow → [Accept] V1 is sync; user waits for completion
- [Risk] Missing artifacts → [Mitigation] Transition lookup already validates required artifacts

## Migration Plan

1. Implement WorkerExecutor in orchestrator (T6001)
2. Add decode functions to openspec crate (T6002)
3. Implement execute_transition (T6003)
4. Implement run_workflow (T6004)
5. Test end-to-end with real API calls

## Open Questions

- Should we cache prompts? → Decision: No for v1; prompts are small
- How to handle partial failures mid-workflow? → Decision: Run stops; user can inspect state
