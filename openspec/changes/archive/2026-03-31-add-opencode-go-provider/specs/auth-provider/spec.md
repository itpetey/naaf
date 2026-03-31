## ADDED Requirements

### Requirement: Auth trait for provider authentication

The system SHALL provide an `Auth` trait that abstracts authentication configuration for LLM providers.

#### Scenario: Auth provides base URL
- **WHEN** a provider needs to construct an API URL
- **THEN** the `Auth` trait provides `base_url(&self) -> &str` returning the provider's base URL

#### Scenario: Auth provides authentication header
- **WHEN** a provider needs to authenticate a request
- **THEN** the `Auth` trait provides `auth_header(&self) -> (&'static str, String)` returning header name and value

### Requirement: OpenAiAuth implementation

The system SHALL provide an `OpenAiAuth` struct implementing `Auth` for OpenAI's API.

#### Scenario: OpenAiAuth base URL
- **WHEN** `OpenAiAuth::new(api_key)` is created
- **THEN** `base_url()` returns "https://api.openai.com"

#### Scenario: OpenAiAuth custom base URL
- **WHEN** `OpenAiAuth::with_base_url(api_key, custom_url)` is created
- **THEN** `base_url()` returns the custom URL

#### Scenario: OpenAiAuth header format
- **WHEN** `auth_header()` is called on `OpenAiAuth`
- **THEN** it returns ("Authorization", "Bearer {api_key}")

### Requirement: OpenCodeAuth implementation

The system SHALL provide an `OpenCodeAuth` struct implementing `Auth` for OpenCode Go's API.

#### Scenario: OpenCodeAuth base URL
- **WHEN** `OpenCodeAuth::new(api_key)` is created
- **THEN** `base_url()` returns "https://opencode.ai/zen/go"

#### Scenario: OpenCodeAuth header format
- **WHEN** `auth_header()` is called on `OpenCodeAuth`
- **THEN** it returns ("Authorization", "Bearer {api_key}")

### Requirement: Auth trait Send + Sync bounds

The `Auth` trait SHALL require `Send + Sync` bounds for thread-safe usage.

#### Scenario: Auth is Send + Sync
- **WHEN** compiling code that stores `Auth` implementations in `Arc<dyn Auth>`
- **THEN** compilation succeeds without trait bound errors