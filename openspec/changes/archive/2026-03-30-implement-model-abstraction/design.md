## Context

The orchestrator needs to call LLM providers to execute workers, but currently has no abstraction for this. The model crate and provider-openai crate are empty placeholders. We need to establish the provider boundary:

1. `model` crate provides: types + trait (no implementation)
2. `orchestrator` depends on: `model` crate only
3. `provider-openai` implements: the trait for OpenAI

This follows the dependency inversion principle - orchestrator depends on abstraction, not concrete implementation.

## Goals / Non-Goals

**Goals:**
- Define provider-neutral types that any LLM can support
- Create a small, practical trait that covers worker execution needs
- Implement OpenAI as the first concrete provider
- Ensure orchestrator never imports provider-openai

**Non-Goals:**
- Support for multiple LLM providers beyond OpenAI in v1
- Streaming responses (sync only for v1)
- Advanced features like function calling, vision, etc.

## Decisions

### Decision 1: Request/Response Types

Define GenerationRequest and GenerationResponse in types.rs:
- GenerationRequest: model, messages (role/content), temperature, max_tokens
- GenerationResponse: content, model, usage info, finish_reason

Rationale: Minimal set needed for worker execution. No vendor-specific fields.

### Decision 2: ModelProvider Trait

Trait signature:
```rust
pub trait ModelProvider: Send + Sync {
    fn generate(&self, request: GenerationRequest) -> Result<GenerationResponse, ProviderError>;
    fn capabilities(&self) -> ProviderCapabilities;
}
```

Rationale: Simple sync method for v1. Future can add async variants.

### Decision 3: ProviderCapabilities

Struct containing:
- supports_streaming: bool
- max_context_tokens: u32
- supported_models: Vec<String>

Rationale: Allows orchestrator to check provider capabilities before calling.

### Decision 4: Error Handling

Define ProviderError enum:
- Authentication(String)
- RateLimited(String)
- ModelNotFound(String)
- InvalidRequest(String)
- NetworkError(String)
- ParseError(String)

Rationale: Specific errors allow better error handling in orchestrator.

### Decision 5: OpenAI Implementation

Use reqwest for HTTP, read API key from OPENAI_API_KEY env var.

Rationale: reqwest is well-established in Rust ecosystem. Environment variable is standard for API keys.

### Decision 6: Orchestrator Integration

Add ModelClient to ExecutionEngine that holds Arc<dyn ModelProvider>.

Rationale: Dependency injection allows swapping providers. Arc enables cheap cloning.

## Risks / Trade-offs

- [Risk] OpenAI API changes → [Mitigation] Map to generic types; update adapter, not orchestrator
- [Risk] API key handling → [Decision] Use env var, not config file for v1
- [Risk] Blocking calls in async context → [Accept] Use blocking reqwest for v1; tokio spawn if needed later

## Migration Plan

1. Implement model crate types and trait (T4001, T4002)
2. Add model dependency to orchestrator Cargo.toml (T4003)
3. Update ExecutionEngine to accept ModelProvider (T4003)
4. Implement provider-openai client skeleton (T4004)
5. Implement first generate() call (T4005)
6. Test end-to-end with mock or real API

## Open Questions

- Should we use async trait methods? → Deferred to future; blocking is simpler for v1
- Should we add retry logic at provider level or orchestrator level? → Orchestrator handles retries via TransitionSpec.retry_limit
