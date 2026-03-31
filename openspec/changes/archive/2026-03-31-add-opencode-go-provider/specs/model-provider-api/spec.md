## ADDED Requirements

### Requirement: Generic Provider struct

The system SHALL provide a generic `Provider<A: Auth, S: ApiSpec>` struct that implements `ModelProvider`.

#### Scenario: Provider implements ModelProvider
- **WHEN** a `Provider<OpenAiAuth, OpenAiChatCompletions>` is created
- **THEN** it implements the `ModelProvider` trait with `generate()` and `capabilities()` methods

#### Scenario: Provider delegates to Auth
- **WHEN** `Provider::generate()` constructs the HTTP request
- **THEN** it uses `Auth::base_url()` and `Auth::auth_header()` for authentication

#### Scenario: Provider delegates to ApiSpec
- **WHEN** `Provider::generate()` builds the request body
- **THEN** it uses `ApiSpec::endpoint()`, `ApiSpec::build_request()`, and `ApiSpec::parse_response()`

### Requirement: Provider compose Auth and ApiSpec

The `Provider` struct SHALL compose `Auth` and `ApiSpec` as orthogonal concerns.

#### Scenario: Same Auth, different ApiSpec
- **WHEN** creating `Provider<OpenCodeAuth, OpenAiChatCompletions>` and `Provider<OpenCodeAuth, AnthropicMessages>`
- **THEN**both compile and work correctly with different endpoints

#### Scenario: Same ApiSpec, different Auth
- **WHEN** creating `Provider<OpenAiAuth, OpenAiChatCompletions>` and `Provider<OpenCodeAuth, OpenAiChatCompletions>`
- **THEN** both compile and work correctly with different base URLs

### Requirement: Factory module for OpenAI models

The system SHALL provide an `openai` module with factory functions for OpenAI models.

#### Scenario: gpt4 factory function
- **WHEN** `openai::gpt4(api_key)` is called
- **THEN** it returns a `Provider<OpenAiAuth, OpenAiChatCompletions>` configured for GPT-4

#### Scenario: gpt35_turbo factory function
- **WHEN** `openai::gpt35_turbo(api_key)` is called
- **THEN** it returns a `Provider<OpenAiAuth, OpenAiChatCompletions>` configured for GPT-3.5 Turbo

### Requirement: Provider thread safety

The `Provider` struct SHALL be `Send + Sync` when both `A: Auth` and `S: ApiSpec` are `Send + Sync`.

#### Scenario: Provider is Send + Sync
- **WHEN** compiling code that stores `Provider` in `Arc<ModelProvider>`
- **THEN** compilation succeeds without trait bound errors