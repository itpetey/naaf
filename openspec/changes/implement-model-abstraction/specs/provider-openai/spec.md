## ADDED Requirements

### Requirement: OpenAiProvider implements ModelProvider trait
The system SHALL provide an OpenAiProvider that implements ModelProvider for OpenAI API.

#### Scenario: Create OpenAiProvider
- **GIVEN** OPENAI_API_KEY environment variable is set
- **WHEN** OpenAiProvider::new() is called
- **THEN** a provider instance is created

#### Scenario: Create OpenAiProvider without API key
- **GIVEN** OPENAI_API_KEY environment variable is not set
- **WHEN** OpenAiProvider::new() is called
- **THEN** a ProviderError::Authentication error is returned

### Requirement: OpenAiProvider calls OpenAI API
The system SHALL translate GenerationRequest to OpenAI API format and call the API.

#### Scenario: Successful generation call
- **GIVEN** a valid GenerationRequest with model="gpt-4" and a user message
- **WHEN** provider.generate(request) is called
- **THEN** OpenAI API is called with correct payload
- **AND** response content is extracted and returned

#### Scenario: API returns error
- **GIVEN** OpenAI API returns an error response
- **WHEN** provider.generate(request) is called
- **THEN** appropriate ProviderError variant is returned

### Requirement: OpenAiProvider maps OpenAI errors to ProviderError
The system SHALL translate OpenAI API errors to provider-neutral errors.

#### Scenario: Map 401 authentication error
- **GIVEN** OpenAI API returns 401
- **WHEN** error is translated
- **THEN** ProviderError::Authentication is returned

#### Scenario: Map 429 rate limit error
- **GIVEN** OpenAI API returns 429
- **WHEN** error is translated
- **THEN** ProviderError::RateLimited is returned

#### Scenario: Map 404 not found error
- **GIVEN** OpenAI API returns 404 (model not found)
- **WHEN** error is translated
- **THEN** ProviderError::ModelNotFound is returned

### Requirement: OpenAiProvider returns correct capabilities
The system SHALL report correct capabilities for the OpenAI provider.

#### Scenario: Check streaming support
- **WHEN** provider.capabilities().supports_streaming is queried
- **THEN** false is returned (v1 does not support streaming)
