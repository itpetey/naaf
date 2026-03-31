## ADDED Requirements

### Requirement: ApiSpec trait for wire format abstraction

The system SHALL provide an `ApiSpec` trait that abstracts LLM API wire formats.

#### Scenario: ApiSpec provides endpoint path
- **WHEN** a provider needs to construct the API endpoint
- **THEN** the `ApiSpec` trait provides `endpoint(&self) -> &'static str` returning the endpoint path

#### Scenario: ApiSpec builds HTTP request body
- **WHEN** a provider needs to send a generation request
- **THEN** the `ApiSpec` trait provides `build_request(&self, GenerationRequest) -> impl Serialize` returning the API-specific request body

#### Scenario: ApiSpec parses HTTP response
- **WHEN** a provider receives an API response
- **THEN** the `ApiSpec` trait provides `parse_response(&self, &str) -> Result<GenerationResponse>` to deserialize the response

#### Scenario: ApiSpec maps API errors
- **WHEN** a provider receives an error response
- **THEN** the `ApiSpec` trait provides `parse_error(&self, u16, &str) -> ProviderError` to map API errors

### Requirement: OpenAiChatCompletions ApiSpec implementation

The system SHALL provide an `OpenAiChatCompletions` struct implementing `ApiSpec` for the OpenAI Chat Completions API.

#### Scenario: OpenAiChatCompletions endpoint
- **WHEN** `endpoint()` is called on `OpenAiChatCompletions`
- **THEN** it returns "/v1/chat/completions"

#### Scenario: OpenAiChatCompletions request format
- **WHEN** `build_request()` is called with a `GenerationRequest`
- **THEN** it returns a serializable struct with `model`, `messages`, `temperature`, and `max_tokens` fields

#### Scenario: OpenAiChatCompletions response format
- **WHEN** `parse_response()` is called with a valid OpenAI response
- **THEN** it returns a `GenerationResponse` with `content`, `model`, `usage`, and `finish_reason`

#### Scenario: OpenAiChatCompletions error mapping
- **WHEN** `parse_error()` is called with status code and error body
- **THEN** it maps HTTP status codes to `ProviderError` variants (401→ Authentication, 404→ ModelNotFound,429→ RateLimited, etc.)

### Requirement: AnthropicMessages ApiSpec implementation

The system SHALL provide an `AnthropicMessages` struct implementing `ApiSpec` for the Anthropic Messages API.

#### Scenario: AnthropicMessages endpoint
- **WHEN** `endpoint()` is called on `AnthropicMessages`
- **THEN** it returns "/v1/messages"

#### Scenario: AnthropicMessages request format
- **WHEN** `build_request()` is called with a `GenerationRequest`
- **THEN** it returns a serializable struct with Anthropic-compatible fields (`model`, `messages`, `max_tokens`, etc.)

#### Scenario: AnthropicMessages response format
- **WHEN** `parse_response()` is called with a valid Anthropic response
- **THEN** it returns a `GenerationResponse` with normalized fields

### Requirement: ApiSpec trait Send + Sync bounds

The `ApiSpec` trait SHALL require `Send + Sync` bounds for thread-safe usage.

#### Scenario: ApiSpec is Send + Sync
- **WHEN** compiling code that stores `ApiSpec` implementations in `Arc<dyn ApiSpec>`
- **THEN** compilation succeeds without trait bound errors