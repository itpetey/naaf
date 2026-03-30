## 1. Model Types (T4001)

- [x] 1.1 Define GenerationRequest struct in types.rs
- [x] 1.2 Define Message struct with role/content
- [x] 1.3 Define GenerationResponse struct
- [x] 1.4 Define ProviderCapabilities struct
- [x] 1.5 Add builder methods or constructors for each type

## 2. Model Provider Trait (T4002)

- [x] 2.1 Define ProviderError enum with variants: Authentication, RateLimited, ModelNotFound, InvalidRequest, NetworkError, ParseError
- [x] 2.2 Define ModelProvider trait with generate() and capabilities() methods
- [x] 2.3 Add Send + Sync bounds to trait
- [x] 2.4 Export types from lib.rs

## 3. Orchestrator Integration (T4003)

- [x] 3.1 Add model crate dependency to orchestrator Cargo.toml
- [x] 3.2 Update ExecutionEngine to accept Arc<dyn ModelProvider>
- [x] 3.3 Verify orchestrator does not import provider-openai
- [x] 3.4 Build to verify compilation

## 4. Provider OpenAI Skeleton (T4004)

- [x] 4.1 Add reqwest dependency to provider-openai Cargo.toml
- [x] 4.2 Define OpenAiConfig to read from OPENAI_API_KEY env var
- [x] 4.3 Implement OpenAiProvider struct
- [x] 4.4 Implement ModelProvider trait for OpenAiProvider
- [x] 4.5 Add error mapping from OpenAI API errors to ProviderError

## 5. First Real Text Generation (T4005)

- [x] 5.1 Implement HTTP POST to OpenAI /v1/chat/completions
- [x] 5.2 Map GenerationRequest to OpenAI request format
- [x] 5.3 Map OpenAI response to GenerationResponse
- [x] 5.4 Add test with mock API or verify manual call works
- [x] 5.5 Handle rate limiting with basic retry

## 6. Tests

- [x] 6.1 Add unit tests for GenerationRequest/Response serialization
- [x] 6.2 Add test for OpenAiProvider construction with missing API key
- [x] 6.3 Add integration test for successful generation call (mock or real)
