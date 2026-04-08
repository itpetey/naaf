## ADDED Requirements

### Requirement: LlmServiceConfig builder

The system SHALL provide an `LlmServiceConfig` builder for constructing `LlmService` instances.

#### Scenario: Config created with API key
- **WHEN** `LlmServiceConfig::new().with_api_key("sk-xxx").build()` is called
- **THEN** a configured `LlmService` is returned ready to make LLM calls

#### Scenario: Config with custom endpoint
- **WHEN** `LlmServiceConfig::new().with_api_key("sk-xxx").with_endpoint("https://custom.example.com").build()` is called
- **THEN** LLM calls are made to the custom endpoint instead of default

### Requirement: Provider type selection

The `LlmServiceConfig` SHALL allow selecting between different LLM provider types.

#### Scenario: OpenAI provider selected
- **WHEN** `LlmServiceConfig::new().provider(ProviderType::OpenAi).with_api_key("sk-xxx").build()` is called
- **THEN** an OpenAI-compatible provider is created

#### Scenario: OpenCode Go provider selected
- **WHEN** `LlmServiceConfig::new().provider(ProviderType::OpenCodeGo).with_api_key("xxx").build()` is called
- **THEN** an OpenCode Go provider is created

### Requirement: Model selection

The `LlmServiceConfig` SHALL allow specifying which model to use.

#### Scenario: Custom model specified
- **WHEN** `LlmServiceConfig::new().with_model("gpt-4-turbo").build()` is called
- **THEN** the specified model is used for all LLM calls

#### Scenario: Default model used when not specified
- **WHEN** `LlmServiceConfig::new().build()` is called without `with_model`
- **THEN** a sensible default model is used

### Requirement: Service creation from config

The `LlmService::from_config()` method SHALL create a service from configuration.

#### Scenario: Service created from config
- **WHEN** `LlmService::from_config(config)` is called
- **THEN** an `LlmService` instance is returned configured with the given settings

### Requirement: DummyServices fallback

The system SHALL allow `DummyServices` to be used for testing.

#### Scenario: DummyServices used in tests
- **WHEN** `ExecCtx::new(run_id, DummyServices::default())` is created
- **THEN** the context works without any real LLM configuration