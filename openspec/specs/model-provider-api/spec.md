## ADDED Requirements

### Requirement: Async generation interface

The `ModelProvider` trait SHALL provide an async `generate` method that returns a `Future` resolving to `Result<GenerationResponse>`.

#### Scenario: Async generate call
- **WHEN** a caller invokes `provider.generate(request).await`
- **THEN** the method returns a `Future` that resolves to `Result<GenerationResponse>` without blocking the async runtime

#### Scenario: Network errors propagated
- **WHEN** the provider encounters a network error during generation
- **THEN** the method returns `Err(ProviderError::NetworkError(_))`

#### Scenario: Authentication errors propagated
- **WHEN** the provider receives an authentication failure from the API
- **THEN** the method returns `Err(ProviderError::Authentication(_))`

### Requirement: Async capabilities discovery

The `ModelProvider` trait SHALL provide an async `capabilities` method that returns a `Future` resolving to `ProviderCapabilities`.

#### Scenario: Async capabilities call
- **WHEN** a caller invokes `provider.capabilities().await`
- **THEN** the method returns a `Future` that resolves to `ProviderCapabilities` without blocking the async runtime

### Requirement: Thread-safe futures

All futures returned by `ModelProvider` trait methods SHALL be `Send`.

#### Scenario: Future requires Send bound
- **WHEN** compiling code that stores the future in a `Box<dyn Future + Send>`
- **THEN** compilation succeeds without trait bound errors

### Requirement: Existing error types preserved

The async migration SHALL preserve the existing `ProviderError` enum and `Result<T>` type alias without modification.

#### Scenario: Error types unchanged
- **WHEN** user code matches on `ProviderError` variants
- **THEN** all existing variants (`Authentication`, `RateLimited`, `ModelNotFound`, `InvalidRequest`, `NetworkError`, `ParseError`) remain available with unchanged semantics

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
- **WHEN** creating `Provider<OpenCodeAuth, OpenAiChatCompletions>` and `Provider<OpenCodeAuth, AnthropicesMessages>`
- **THEN** both compile and work correctly with different endpoints

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