## ADDED Requirements

### Requirement: GenerationRequest defines text generation parameters
The system SHALL provide a GenerationRequest struct that contains all parameters needed for text generation.

#### Scenario: Create request with minimal parameters
- **GIVEN** a model identifier and user message
- **WHEN** GenerationRequest is constructed
- **THEN** it SHALL contain: model, messages, temperature (default 0.7), max_tokens (default 1024)

#### Scenario: Create request with custom parameters
- **GIVEN** model, messages, temperature=0.9, max_tokens=500
- **WHEN** GenerationRequest is constructed
- **THEN** all custom values are stored in the struct

### Requirement: Message struct supports role and content
The system SHALL provide a Message struct with role and content fields.

#### Scenario: Create user message
- **WHEN** Message::user("hello") is called
- **THEN** role is "user" and content is "hello"

#### Scenario: Create system message
- **WHEN** Message::system("You are helpful.") is called
- **THEN** role is "system" and content is "You are helpful."

### Requirement: GenerationResponse contains generation output
The system SHALL provide a GenerationResponse struct with the generated content.

#### Scenario: Successful response
- **GIVEN** a successful generation
- **WHEN** GenerationResponse is received
- **THEN** it SHALL contain: content (String), model (String), usage (token counts), finish_reason

### Requirement: ProviderCapabilities describes provider features
The system SHALL provide a ProviderCapabilities struct.

#### Scenario: Check streaming support
- **GIVEN** a ProviderCapabilities with supports_streaming=false
- **WHEN** .supports_streaming() is called
- **THEN** false is returned
