## 1. Add LlmService and Config to core budget module

- [x] 1.1 Add `LlmServiceConfig` builder struct to `crates/core/src/budget.rs`
- [x] 1.2 Add `ProviderType` enum (OpenAi, OpenCodeGo) to budget module
- [x] 1.3 Implement `LlmService::from_config()` that creates provider from config
- [x] 1.4 Ensure `LlmService` implements `Services` trait with proper request/response serialization

## 2. Integrate providers crate dependency

- [x] 2.1 Add `naaf-providers` as dependency to `crates/core/Cargo.toml`
- [x] 2.2 Export necessary types from providers for `LlmService` construction
- [x] 2.3 Handle feature flags for openai/opencode-go providers

## 3. Update execution context and executor

- [x] 3.1 Update `ExecCtx::new()` to accept any `S: Services` (already generic, verify)
- [x] 3.2 Add `with_services()` method for runtime service replacement
- [x] 3.3 Update workflow test code to use `DummyServices` explicitly where needed

## 4. Update workflow definitions to accept configurable services

- [x] 4.1 Update `workflows/openspec/src/workflows.rs` to accept services as parameter
- [x] 4.2 Update `draft_request_workflow()` to not hardcode `DummyServices`
- [x] 4.3 Update other workflow files that create `ExecCtx<DummyServices>`

## 5. Update TUI to use real services in production

- [x] 5.1 Update `crates/tui/src/main.rs` to accept provider configuration
- [x] 5.2 Add CLI flags for API key, endpoint, provider type, model
- [x] 5.3 Create `LlmService` from config in TUI main
- [x] 5.4 Keep `DummyServices` as fallback when no config provided

## 6. Verify and test

- [x] 6.1 Run `cargo fmt --all`
- [x] 6.2 Run `cargo clippy -- -D warnings`
- [x] 6.3 Run `cargo test` to ensure existing tests still pass
- [x] 6.4 Verify workflow execution works with both DummyServices and LlmService
