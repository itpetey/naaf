## Why

The codebase currently uses `DummyServices` - a test mock that provides deterministic, fake LLM responses - throughout the workflow system. This was appropriate for early development and testing, but now limits the system's ability to handle real-world inputs and demonstrate genuine AI capabilities. The `providers` crate already contains implementations for OpenAI and OpenCode Go LLM providers, but they aren't wired into the execution path.

## What Changes

- Replace `DummyServices` with a real `LlmService` implementation that uses actual LLM providers
- Add provider configuration infrastructure (API keys, endpoint URLs, model selection)
- Update the workflow execution context to accept a configurable provider
- Add support for multiple provider backends (OpenAI, OpenCode Go) via the existing `providers` crate
- Maintain test compatibility by allowing `DummyServices` fallback in test environments

## Capabilities

### New Capabilities
- `llm-service`: Core service that provides real LLM generation via the `providers` crate
- `provider-configuration`: Configuration system for specifying which LLM provider and model to use

### Modified Capabilities
- `execution-context`: Update to support real `LlmService` instead of `DummyServices`
- `workflow-executor`: Update to use configurable provider instead of hardcoded `DummyServices`

## Impact

- Changes to `crates/core/src/budget.rs` - Add `LlmService` type and update execution context
- Changes to `workflows/openspec/src/*.rs` - Replace `DummyServices` with real service in workflow definitions
- Changes to `crates/providers/` - Wire in the existing provider implementations
- New dependency: `naaf-providers` crate integrated into workflow execution