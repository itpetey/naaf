## ADDED Requirements

### Requirement: LlmService implements Services trait

The `LlmService` struct SHALL implement the `Services` trait, enabling it to be used as the services implementation in `ExecCtx<S>`.

#### Scenario: LlmService used in ExecCtx
- **WHEN** an `ExecCtx<LlmService>` is created with a configured provider
- **THEN** the context can make LLM calls via the services field

#### Scenario: LlmService delegates to provider
- **WHEN** `services.call("llm", request).await` is invoked
- **THEN** the request is forwarded to the underlying `ModelProvider` and response is returned

### Requirement: LlmService wraps ModelProvider

The `LlmService` SHALL wrap a `Box<dyn ModelProvider>` to provide flexibility in provider selection.

#### Scenario: OpenAI provider configured
- **WHEN** `LlmService::new(Box::new(openai_provider))` is called
- **THEN** LLM calls are made to OpenAI API

#### Scenario: OpenCode provider configured
- **WHEN** `LlmService::new(Box::new(opencode_provider))` is called
- **THEN** LLM calls are made to OpenCode Go API

### Requirement: Service name routing

The `LlmService` SHALL route service calls based on the service name parameter.

#### Scenario: LLM service call
- **WHEN** `services.call("llm", request).await` is invoked
- **THEN** the request is parsed and forwarded to the model provider

#### Scenario: Unknown service name
- **WHEN** `services.call("unknown", request).await` is invoked
- **THEN** an error is returned indicating unknown service

### Requirement: Request/response serialization

The `LlmService` SHALL serialize requests to JSON and deserialize responses from JSON.

#### Scenario: Request serialized to JSON
- **WHEN** a request is passed to `call()`
- **THEN** the request is serialized as JSON before being sent to the provider

#### Scenario: Response deserialized from JSON
- **WHEN** the provider returns a response
- **THEN** the response is deserialized from JSON and returned to caller

### Requirement: Error propagation

The `LlmService` SHALL propagate errors from the underlying provider.

#### Scenario: Network error propagated
- **WHEN** the provider encounters a network error
- **THEN** `LlmService` returns `Err(ProviderError::NetworkError(_))`

#### Scenario: Authentication error propagated
- **WHEN** the provider receives an authentication failure
- **THEN** `LlmService` returns `Err(ProviderError::Authentication(_))`