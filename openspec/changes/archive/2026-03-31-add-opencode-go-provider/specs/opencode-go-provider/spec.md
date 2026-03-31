## ADDED Requirements

### Requirement: Factory module for OpenCode Go models

The system SHALL provide an `opencode_go` module with factory functions for each supported model.

#### Scenario: glm5 factory function
- **WHEN** `opencode_go::glm5(api_key)` is called
- **THEN** it returns a `Provider<OpenCodeAuth, OpenAiChatCompletions>` configured for GLM-5

#### Scenario: kimi_k25 factory function
- **WHEN** `opencode_go::kimi_k25(api_key)` is called
- **THEN** it returns a `Provider<OpenCodeAuth, OpenAiChatCompletions>` configured for Kimi K2.5

#### Scenario: minimax_m25 factory function
- **WHEN** `opencode_go::minimax_m25(api_key)` is called
- **THEN** it returns a `Provider<OpenCodeAuth, AnthropicesMessages>` configured for MiniMax M2.5

#### Scenario: minimax_m27 factory function
- **WHEN** `opencode_go::minimax_m27(api_key)` is called
- **THEN** it returns a `Provider<OpenCodeAuth, AnthropicMessages>` configured for MiniMax M2.7

### Requirement: Factory function ModelProvider implementation

Each factory function SHALL return a type that implements `ModelProvider`.

#### Scenario: Factory return type implements ModelProvider
- **WHEN** calling `generate()` on the result of `opencode_go::glm5(api_key)`
- **THEN** the call succeeds and returns `Result<GenerationResponse>`

#### Scenario: Factory return type implements capabilities
- **WHEN** calling `capabilities()` on the result of `opencode_go::glm5(api_key)`
- **THEN** the call succeeds and returns `ProviderCapabilities` with the correct model name

### Requirement: Model name configuration

Factory functions SHALL configure the provider with the correct model identifier.

#### Scenario: GLM-5 model name
- **WHEN** `glm-5` model is used via factory function
- **THEN** requests include `model: "glm-5"` in the request body

#### Scenario: Kimi K2.5 model name
- **WHEN** `kimi-k2.5` model is used via factory function
- **THEN** requests include `model: "kimi-k2.5"` in the request body

#### Scenario: MiniMax M2.5 model name
- **WHEN** `minimax-m2.5` model is used via factory function
- **THEN** requests include the correct model identifier for Anthropic Messages API

#### Scenario: MiniMax M2.7 model name
- **WHEN** `minimax-m2.7` model is used via factory function
- **THEN** requests include the correct model identifier for Anthropices Messages API