## 1. Trait Definition Update

- [x] 1.1 Update `ModelProvider` trait in `crates/model/src/provider.rs` to use async `generate` method
- [x] 1.2 Update `ModelProvider` trait to use async `capabilities` method
- [x] 1.3 Ensure futures returned by trait methods have `Send` bound
- [x] 1.4 Run `cargo clippy -- -D warnings` to verify trait compiles correctly

## 2. OpenAI Provider Implementation

- [x] 2.1 Replace `reqwest::blocking::Client` with async `reqwest::Client` in `OpenAiProvider`
- [x] 2.2 Update `generate` implementation to use async reqwest methods with `.await`
- [x] 2.3 Update `capabilities` implementation to return async future
- [x] 2.4 Update integration tests to use `#[tokio::test]`
- [x] 2.5 Run `cargo test -p provider-openai` to verify OpenAI implementation

## 3. Mock Provider Updates

- [x] 3.1 Update `MockProvider` in `crates/orchestrator/src/workflow.rs` to implement async trait
- [x] 3.2 Update `MockProvider` in `crates/orchestrator/src/remediation.rs` to implement async trait
- [x] 3.3 Verify mock implementations compile and pass tests

## 4. Orchestrator Callers

- [x] 4.1 Update `workflow.rs` call sites to use `.await` for `generate()` calls
- [x] 4.2 Update any other callers in `orchestrator` crate to use async syntax
- [x] 4.3 Run `cargo test -p orchestrator` to verify orchestrator compiles and passes

## 5. Verification

- [x] 5.1 Run `cargo fmt --all` to format all changes
- [x] 5.2 Run `cargo clippy -- -D warnings` across workspace
- [x] 5.3 Run `cargo test` to verify all tests pass
- [x] 5.4 Run `cargo build --release` to verify release build succeeds