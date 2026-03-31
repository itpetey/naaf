## Why

The `ModelProvider` trait uses synchronous methods, but model providers (like OpenAI) perform network I/O that should be async. Synchronous blocking calls reduce performance in async runtimes and prevent proper resource utilisation. Making the trait async aligns with Rust's async ecosystem and enables efficient non-blocking I/O for all provider implementations.

## What Changes

- **BREAKING**: `ModelProvider::generate()` becomes `async fn generate()`
- **BREAKING**: `ModelProvider::capabilities()` becomes `async fn capabilities()`
- **BREAKING**: Replace all `dyn ModelProvider` with generic parameters `P: ModelProvider`
- Update `OpenAiProvider` implementation to use async reqwest
- Update `MockProvider` implementations in orchestrator tests
- Update all callers to use `.await` syntax
- Replace `reqwest::blocking::Client` with async `reqwest::Client`
- Make `ModelClient`, `WorkerExecutor`, `DefaultExecutionEngine`, `RemediationEngine` generic over provider type

## Capabilities

### New Capabilities

- `model-provider-api`: Defines the async interface contract for model providers, including error handling and capability discovery

### Modified Capabilities

(None - this is a new capability specification)

## Impact

- `crates/model/src/provider.rs` - trait definition
- `crates/provider-openai/src/client.rs` - OpenAI implementation
- `crates/orchestrator/src/workflow.rs` - caller of `generate()`
- `crates/orchestrator/src/remediation.rs` - MockProvider in tests
- Downstream crates that depend on `model` will need to update to async call patterns