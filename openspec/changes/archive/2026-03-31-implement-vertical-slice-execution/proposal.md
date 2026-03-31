## Why

The orchestrator has the execution engine, the model provider abstraction, and OpenSpec workers, but they haven't been wired together yet. We need to implement the prompt execution path that transforms worker specs into model calls, parses responses back into artifacts, and executes the full happy-path workflow. This is the critical integration that proves the architecture works end-to-end.

## What Changes

- **T6001**: Implement prompt execution adapter in orchestrator (render prompt + call model)
- **T6002**: Implement typed decoding for OpenSpec worker outputs (parse LLM response -> artifact)
- **T6003**: Execute one transition end-to-end (UserPrompt -> NormalizedSpec)
- **T6004**: Execute the happy-path workflow end-to-end (all 4 transitions)

## Capabilities

### New Capabilities

- `prompt-execution-adapter`: Path from worker spec to model call
- `worker-output-decoding`: Parse LLM responses into OpenSpec artifacts
- `transition-execution`: Execute single transition with persistence
- `workflow-runner`: Execute full happy-path workflow

### Modified Capabilities

- (none - new capabilities only)

## Impact

- **Code affected**: `orchestrator` crate (prompt adapter), `openspec` crate (decoding)
- **Dependencies**: Requires Phase 3-5 to be complete
- **First end-to-end test**: This is the first real execution proof
