## Why

OpenCode Go is a subscription service providing access to open coding models (GLM-5, Kimi K2.5, MiniMax M2.5/M2.7) through OpenAI-compatible and Anthropic-compatible APIs. Creating a provider for OpenCode Go would allow naaf to use these models. Additionally, separating authentication from API specifications enables reuse across multiple providers that share the same API wire format.

## What Changes

- Add `Auth` trait for provider authentication (base URL + auth headers)
- Add `ApiSpec` trait for API wire formats (request/response shapes + endpoints)
- Add generic `Provider<A, S>` struct implementing `ModelProvider`
- Add `OpenAiAuth` and `OpenCodeAuth` implementations
- Add `OpenAiChatCompletions` and `AnthropicMessages` ApiSpec implementations
- Add factory modules `openai` and `opencode_go` with typed model constructors
- Deprecate monolithic `providers::openai` module in favor of new architecture

## Capabilities

### New Capabilities

- `auth-provider`: Authentication abstraction for LLM providers (Auth trait, OpenAiAuth, OpenCodeAuth)
- `api-spec`: Wire format abstraction for LLM APIs (ApiSpec trait, OpenAiChatCompletions, AnthropicesMessages)
- `opencode-go-provider`: Factory functions for OpenCode Go models (glm-5, kimi-k2.5, minimax-m2.5, minimax-m2.7)

### Modified Capabilities

- `model-provider-api`: Add guidance on implementing ModelProvider via generic Provider<A, S> composition

## Impact

- `crates/providers/src/lib.rs` - Update exports to use new module structure
- `crates/providers/src/openai.rs` - Replace with factory module using new architecture
- New files under `crates/providers/src/auth/`, `crates/providers/src/api/`, `crates/providers/src/provider.rs`
- `crates/model/src/provider.rs` - No changes (trait remains unchanged)
- Tests will need to use new factory constructors