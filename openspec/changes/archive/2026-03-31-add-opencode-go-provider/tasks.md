## 1. Auth Trait and Implementations

- [x] 1.1 Create `crates/providers/src/auth/mod.rs` with `Auth` trait definition
- [x] 1.2 Create `crates/providers/src/auth/openai.rs` with `OpenAiAuth` struct
- [x] 1.3 Create `crates/providers/src/auth/opencode.rs` with `OpenCodeAuth` struct
- [x] 1.4 Add unit tests for `Auth` trait implementations
- [x] 1.5 Export auth module from `crates/providers/src/lib.rs`

## 2. ApiSpec Trait and Implementations

- [x] 2.1 Create `crates/providers/src/api/mod.rs` with `ApiSpec` trait definition
- [x] 2.2 Create `crates/providers/src/api/openai_chat.rs` with `OpenAiChatCompletions` struct
- [x] 2.3 Create `crates/providers/src/api/anthropic.rs` with `AnthropicMessages` struct
- [x] 2.4 Add unit tests for `ApiSpec` trait implementations
- [x] 2.5 Export api module from `crates/providers/src/lib.rs`

## 3. Generic Provider Struct

- [x] 3.1 Create `crates/providers/src/provider.rs` with generic `Provider<A, S>` struct
- [x] 3.2 Implement `ModelProvider` trait for `Provider<A, S>`
- [x] 3.3 Add constructor `Provider::new(auth, api_spec)`
- [x] 3.4 Add unit tests for generic Provider composition

## 4. OpenAI Factory Module

- [x] 4.1 Create `crates/providers/src/openai.rs` factory module (replace existing)
- [x] 4.2 Implement `openai::gpt4(api_key)` factory function
- [x] 4.3 Implement `openai::gpt35_turbo(api_key)` factory function
- [x] 4.4 Add `openai::with_base_url()` for custom base URLs
- [x] 4.5 Update existing tests to use new factory functions
- [x] 4.6 Remove old `OpenAiProvider` struct and `OpenAiConfig`

## 5. OpenCode Go Factory Module

- [x] 5.1 Create `crates/providers/src/opencode_go.rs` factory module
- [x] 5.2 Implement `opencode_go::glm5(api_key)` for GLM-5
- [x] 5.3 Implement `opencode_go::kimi_k25(api_key)` for Kimi K2.5
- [x] 5.4 Implement `opencode_go::minimax_m25(api_key)` for MiniMax M2.5
- [x] 5.5 Implement `opencode_go::minimax_m27(api_key)` for MiniMax M2.7
- [x] 5.6 Add integration tests with mocked HTTP responses

## 6. Cleanup and Documentation

- [x] 6.1 Update `crates/providers/Cargo.toml` features for new modules
- [x] 6.2 Add doc comments for all public types
- [x] 6.3 Update module-level documentation in `lib.rs`
- [x] 6.4 Run `cargo fmt --all` and `cargo clippy -- -D warnings`
- [x] 6.5 Run `cargo test` to verify all tests pass