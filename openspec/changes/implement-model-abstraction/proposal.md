## Why

The orchestrator currently has no way to call LLM providers - it only has placeholder modules. We need to establish a provider-neutral abstraction that allows the orchestrator to work with any LLM provider while implementing OpenAI as the first concrete provider. This is essential for Phase 6 vertical slice execution.

## What Changes

- **T4001**: Define provider-neutral model request/response types in model crate
- **T4002**: Define ModelProvider trait in model crate
- **T4003**: Wire orchestrator to depend on model trait only (not concrete provider)
- **T4004**: Implement provider-openai configuration and client skeleton
- **T4005**: Implement first real text generation call via OpenAI

## Capabilities

### New Capabilities

- `model-types`: Provider-neutral request/response types for text generation
- `model-provider-trait`: Abstract interface for LLM providers
- `provider-openai`: Concrete OpenAI provider implementation
- `orchestrator-model-integration`: Wiring orchestrator to use model trait

### Modified Capabilities

- (none - new capabilities only)

## Impact

- **Code affected**: `model` crate (new types + trait), `provider-openai` crate (implementation), `orchestrator` crate (dependency update)
- **Dependencies**: Adds reqwest for HTTP calls in provider-openai
- **Breaking**: Orchestrator will depend on `model` crate (not provider-openai)
