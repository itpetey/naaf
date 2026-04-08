## Context

The workflow system currently uses `DummyServices` - a test mock that returns empty responses - for all workflow executions. The `providers` crate (`crates/providers/`) already has implementations for OpenAI and OpenCode Go LLM providers, but they are not wired into the execution path.

The `Services` trait in `crates/core/src/budget.rs` defines the interface for service calls:
```rust
pub trait Services: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;
    fn call(&self, service: &str, request: &[u8]) -> impl Future<Output = Result<Vec<u8>, Self::Error>> + Send;
}
```

`DummyServices` implements this trait but always returns empty results. There's already a `LlmServices<S>` wrapper defined but it's not being used.

## Goals / Non-Goals

**Goals:**
- Replace `DummyServices` with a real `LlmService` implementation that uses the `providers` crate
- Make the LLM provider configurable (API key, endpoint, model selection)
- Support multiple provider backends (OpenAI, OpenCode Go)
- Maintain test compatibility by allowing `DummyServices` fallback in test code

**Non-Goals:**
- Add provider authentication UI - configuration will be done programmatically
- Implement new LLM provider adapters - OpenAI and OpenCode Go already exist
- Add caching or rate limiting at the provider level
- Change the `Services` trait interface

## Decisions

### Decision 1: Use existing Services trait with LlmServices wrapper

**Rationale:** The `Services` trait already exists and `LlmServices<S>` wrapper is already defined. We can wrap the provider behind this interface without changing the trait. This avoids modifying all the workflow code that depends on `Services`.

**Alternative considered:** Define a new `LlmProvider` trait. Rejected because it would require changing every workflow step that currently uses `Services`.

### Decision 2: Configuration via builder pattern

**Rationale:** Use a builder pattern to allow flexible configuration of providers without requiring complex configuration files. Users can specify API keys, endpoints, and models programmatically.

**Alternative considered:** Environment variables. Rejected because builder pattern is more explicit and testable.

### Decision 3: Keep DummyServices for tests

**Rationale:** Some tests rely on deterministic responses from `DummyServices`. Rather than modify all tests, we keep `DummyServices` available for test code while using real services in production.

**Alternative considered:** Add a test mode flag. Rejected because builder pattern naturally allows passing `DummyServices` for tests.

## Risks / Trade-offs

- **[Risk]** Network failures during LLM calls → **[Mitigation]** Add proper error handling in workflow steps; allow graceful degradation
- **[Risk]** API key exposure in logs → **[Mitigation]** Ensure provider implementation doesn't log sensitive headers
- **[Risk]** Provider API changes breaking compilation → **[Mitigation]** Use trait objects (`dyn ModelProvider`) to decouple from specific provider implementations
- **[Trade-off]** Real LLM calls are slower than mock responses → Accept this for production use; tests continue to use `DummyServices`

## Migration Plan

1. Add `LlmServiceConfig` builder in `crates/core/src/budget.rs`
2. Create `create_provider()` function that builds a provider from config
3. Add `LlmService` that wraps the provider and implements `Services`
4. Update TUI to accept provider configuration and use `LlmService`
5. Update workflow code to accept configurable services (change function signatures)
6. Run existing tests (they will use `DummyServices` via existing test helpers)
7. Add integration test with real provider (optional, behind feature flag)

## Open Questions

- Should we add environment variable fallback for API keys?
- Should we support provider fallback (try OpenAI, fall back to OpenCode Go)?
- Should we add a mock mode for development without real API keys?