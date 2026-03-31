## Context

The current `providers` crate has a monolithic `OpenAiProvider` that couples authentication (API key, base URL), API wire format (OpenAI chat completions), and model capabilities. This design works for a single provider but doesn't scale when:

1. Multiple providers share the same API wire format (OpenAI, OpenCode Go, local models)
2. A single provider offers multiple API formats (OpenCode Go offers both OpenAI-compatible and Anthropic-compatible endpoints)
3. Adding new providers requires duplicating wire format code

The proposal introduces two orthogonal abstractions:
- **Auth**: Base URL + authentication headers
- **ApiSpec**: Request/response wire format + endpoint path

These compose via a generic `Provider<A, S>` that implements `ModelProvider`.

## Goals / Non-Goals

**Goals:**
- Create `Auth` trait with `OpenAiAuth` and `OpenCodeAuth` implementations
- Create `ApiSpec` trait with `OpenAiChatCompletions` and `AnthropicMessages` implementations
- Create generic `Provider<A: Auth, S: ApiSpec>` implementing `ModelProvider`
- Add factory modules `openai` and `opencode_go` with typed model constructors
- Support OpenCode Go models: glm-5, kimi-k2.5(minimax-m2.5, minimax-m2.7)

**Non-Goals:**
- Streaming support (future work)
- Token counting utilities
- Client retry/backoff logic
- Caching layer

## Decisions

### Decision: Trait-based composition over enum dispatch

**Chosen:** Generic `Provider<A: Auth, S: ApiSpec>` with static dispatch

**Alternatives considered:**
- **Enum dispatch:** `ProviderKind` enum with runtime matching. Simpler but requires boxing and dynamic dispatch. No compile-time model safety.
- **Trait object:** `Box<dyn ModelProvider>`. Similar drawbacks to enum approach.

**Rationale:** Static dispatch enables:
- Zero-cost abstraction (monomorphized at compile time)
- Type-safe model constructors (impossible to create invalid Auth+ApiSpec combinations at runtime)
- Compiler catches configuration errors

### Decision: Factory functions over builder pattern

**Chosen:** Module-level factory functions (e.g., `opencode_go::glm5(api_key)`)

**Alternatives considered:**
- **Builder pattern:** `Provider::new(OpenCodeAuth::new(api_key)).with_api(AnthropicMessages)`. More flexible but allows invalid combinations.
- **Provider trait impl:** Each model as a separate struct. Maximum type safety but massive code duplication.

**Rationale:** Factory functions:
- Enforce valid Auth+ApiSpec combinations
- Clear API surface: `opencode_go::glm5()`, `opencode_go::minimax_m27()`
- Easy to discover all supported models via module documentation

### Decision: Auth stores base URL, ApiSpec stores endpoint path

**Chosen:** Auth owns base URL, ApiSpec owns endpoint path

```rust
trait Auth {
    fn base_url(&self) -> &str;
    fn auth_header(&self) -> (&'static str, String);
}

trait ApiSpec {
    fn endpoint(&self) -> &'static str;
    // build_request, parse_response, etc.
}
```

**Alternatives considered:**
- **Full URL in Auth:** Auth provides complete URL. Problem: ApiSpec can't override path for different endpoints.
- **Full URL in ApiSpec:** ApiSpec owns full URL. Problem: Auth can't override base URL for testing/staging.

**Rationale:** Separation enables:
- Testing against mock servers (swap Auth base URL)
- Supporting multiple API versions (same Auth, different ApiSpec)
- Clear ownership: Auth =where/Auth, ApiSpec = what path

## Risks / Trade-offs

### Risk: API drift between OpenAI-compatible providers

**Risk:** OpenCode Go's OpenAI-compatible endpoint may diverge from official OpenAI API.

**Mitigation:** Version the `OpenAiChatCompletions` ApiSpec if differences emerge. Factory functions can select appropriate version per provider.

### Risk: Anthropic Messages API differences

**Risk:** OpenCode Go's Anthropic-compatible endpoint may differ from official Anthropic API.

**Mitigation:** Start with standard Anthropic Messages spec. Create `OpenCodeAnthropicMessages` variant if needed.

### Trade-off: Compile-time model selection

**Trade-off:** Model choice is compile-time, not runtime configurable.

**Acceptance:** This is intentional. Runtime model switching can be added later via enum dispatch or configuration without changing the trait hierarchy.