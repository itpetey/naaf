use naaf_schema::state::RunId;
use naaf_schema::state::StateEnvelope;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use tokio_util::sync::CancellationToken;

use naaf_providers::{
    GenerationRequest, GenerationResponse, Message, ModelProvider, ProviderError,
    Result as ProviderResult,
};

use crate::events::TraceSink;

pub type StepBudget = u32;
pub type BranchBudget = u32;
pub type TokenBudget = u64;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct BudgetState {
    pub max_steps: Option<StepBudget>,
    pub max_branches: Option<BranchBudget>,
    pub token_budget: Option<TokenBudget>,
    pub time_budget_ms: Option<u64>,
}

impl BudgetState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_max_steps(mut self, steps: StepBudget) -> Self {
        self.max_steps = Some(steps);
        self
    }

    pub fn with_max_branches(mut self, branches: BranchBudget) -> Self {
        self.max_branches = Some(branches);
        self
    }

    pub fn with_token_budget(mut self, tokens: TokenBudget) -> Self {
        self.token_budget = Some(tokens);
        self
    }

    pub fn with_time_budget_ms(mut self, ms: u64) -> Self {
        self.time_budget_ms = Some(ms);
        self
    }
}

pub trait Budget {
    fn state(&self) -> &BudgetState;

    fn step_limit(&self) -> Option<StepBudget> {
        self.state().max_steps
    }

    fn branch_limit(&self) -> Option<BranchBudget> {
        self.state().max_branches
    }

    fn token_limit(&self) -> Option<TokenBudget> {
        self.state().token_budget
    }

    fn time_limit_ms(&self) -> Option<u64> {
        self.state().time_budget_ms
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BudgetImpl {
    state: BudgetState,
}

impl BudgetImpl {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_state(mut self, state: BudgetState) -> Self {
        self.state = state;
        self
    }
}

impl Budget for BudgetImpl {
    fn state(&self) -> &BudgetState {
        &self.state
    }
}

pub trait Services: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    fn call(
        &self,
        service: &str,
        request: &[u8],
    ) -> impl std::future::Future<Output = Result<Vec<u8>, Self::Error>> + Send;
}

#[cfg(test)]
#[derive(Clone, Default)]
pub struct DummyServices;

#[cfg(test)]
impl Services for DummyServices {
    type Error = std::io::Error;

    async fn call(&self, _: &str, _: &[u8]) -> Result<Vec<u8>, Self::Error> {
        Ok(vec![])
    }
}

/// Production services that wraps a model provider for actual LLM calls.
pub struct LlmServices<S: Services> {
    provider: S,
}

impl<S: Services> LlmServices<S> {
    pub fn new(provider: S) -> Self {
        Self { provider }
    }
}

impl<S: Services + Send + Sync> Services for LlmServices<S> {
    type Error = S::Error;

    async fn call(&self, service: &str, request: &[u8]) -> Result<Vec<u8>, Self::Error> {
        self.provider.call(service, request).await
    }
}

#[cfg(feature = "openai")]
pub use naaf_providers::openai::OpenAiModel;
#[cfg(feature = "opencode-go")]
pub use naaf_providers::opencode_go::OpenCodeGoModel;

trait DynModelProvider: Send + Sync {
    fn generate<'a>(
        &'a self,
        request: GenerationRequest,
    ) -> Pin<Box<dyn Future<Output = ProviderResult<GenerationResponse>> + Send + 'a>>;
}

impl<T> DynModelProvider for T
where
    T: ModelProvider + Send + Sync,
{
    fn generate<'a>(
        &'a self,
        request: GenerationRequest,
    ) -> Pin<Box<dyn Future<Output = ProviderResult<GenerationResponse>> + Send + 'a>> {
        Box::pin(ModelProvider::generate(self, request))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ProviderType {
    OpenAi,
    #[default]
    OpenCodeGo,
}

#[derive(Debug, Clone)]
pub struct LlmServiceConfig {
    provider_type: ProviderType,
    api_key: Option<String>,
    endpoint: Option<String>,
    model: Option<String>,
}

impl LlmServiceConfig {
    pub fn new() -> Self {
        Self {
            provider_type: ProviderType::default(),
            api_key: None,
            endpoint: None,
            model: None,
        }
    }

    pub fn provider(mut self, provider_type: ProviderType) -> Self {
        self.provider_type = provider_type;
        self
    }

    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    #[cfg(feature = "openai")]
    pub fn build_openai(self) -> ProviderResult<LlmService> {
        use naaf_providers::Provider;
        use naaf_providers::api::OpenAiChatCompletions;
        use naaf_providers::auth::OpenAiAuth;

        let api_key = self
            .api_key
            .ok_or_else(|| ProviderError::InvalidRequest("API key required".into()))?;

        let model = self.model.unwrap_or_else(|| "gpt-5".to_string());
        let provider_model = match model.as_str() {
            "gpt-5" => OpenAiModel::Gpt5,
            "gpt-54" => OpenAiModel::Gpt54,
            other => {
                return Err(ProviderError::InvalidRequest(format!(
                    "Unsupported OpenAI model: {other}"
                )));
            }
        };

        let auth = if let Some(endpoint) = self.endpoint {
            OpenAiAuth::with_base_url(api_key, endpoint)
        } else {
            OpenAiAuth::new(api_key)
        };

        let api = OpenAiChatCompletions::new(provider_model);
        let provider = Provider::new(auth, api);

        Ok(LlmService::new(Box::new(provider), model))
    }

    #[cfg(not(feature = "openai"))]
    pub fn build_openai(self) -> ProviderResult<LlmService> {
        Err(ProviderError::InvalidRequest(
            "OpenAI support not enabled".into(),
        ))
    }

    #[cfg(feature = "opencode-go")]
    pub fn build_opencode_go(self) -> ProviderResult<LlmService> {
        use naaf_providers::Provider;
        use naaf_providers::api::{AnthropicMessages, OpenAiChatCompletions};
        use naaf_providers::auth::OpenCodeAuth;

        let api_key = self
            .api_key
            .ok_or_else(|| ProviderError::InvalidRequest("API key required".into()))?;

        let model = self.model.unwrap_or_else(|| "glm-5".to_string());
        let auth = if let Some(endpoint) = self.endpoint {
            OpenCodeAuth::with_base_url(api_key, endpoint)
        } else {
            OpenCodeAuth::new(api_key)
        };

        match model.as_str() {
            "kimi-k2.5" => {
                let api = OpenAiChatCompletions::new(OpenCodeGoModel::KimiK25);
                let provider = Provider::new(auth, api);
                Ok(LlmService::new(Box::new(provider), model))
            }
            "minimax-m2.5" => {
                let api = AnthropicMessages::new(OpenCodeGoModel::MiniMaxM25);
                let provider = Provider::new(auth, api);
                Ok(LlmService::new(Box::new(provider), model))
            }
            "minimax-m2.7" => {
                let api = AnthropicMessages::new(OpenCodeGoModel::MiniMaxM27);
                let provider = Provider::new(auth, api);
                Ok(LlmService::new(Box::new(provider), model))
            }
            "glm-5" => {
                let api = OpenAiChatCompletions::new(OpenCodeGoModel::Glm5);
                let provider = Provider::new(auth, api);
                Ok(LlmService::new(Box::new(provider), model))
            }
            other => Err(ProviderError::InvalidRequest(format!(
                "Unsupported OpenCode Go model: {other}"
            ))),
        }
    }

    #[cfg(not(feature = "opencode-go"))]
    pub fn build_opencode_go(self) -> ProviderResult<LlmService> {
        Err(ProviderError::InvalidRequest(
            "OpenCode Go support not enabled".into(),
        ))
    }

    pub fn build(self) -> ProviderResult<LlmService> {
        match self.provider_type {
            ProviderType::OpenAi => self.build_openai(),
            ProviderType::OpenCodeGo => self.build_opencode_go(),
        }
    }
}

impl Default for LlmServiceConfig {
    fn default() -> Self {
        Self::new()
    }
}

pub struct LlmService {
    provider: Box<dyn DynModelProvider>,
    model: String,
}

impl LlmService {
    fn new(provider: Box<dyn DynModelProvider>, model: String) -> Self {
        Self { provider, model }
    }

    pub fn from_config(config: LlmServiceConfig) -> ProviderResult<Self> {
        config.build()
    }
}

impl Services for LlmService {
    type Error = ProviderError;

    async fn call(&self, service: &str, request: &[u8]) -> Result<Vec<u8>, Self::Error> {
        if service != "llm" {
            return Err(ProviderError::InvalidRequest(format!(
                "Unknown service: {}",
                service
            )));
        }

        let prompt = std::str::from_utf8(request)
            .map_err(|e| ProviderError::ParseError(e.to_string()))?
            .to_string();
        let gen_request = GenerationRequest::new(self.model.clone(), vec![Message::user(prompt)]);
        let response = self.provider.generate(gen_request).await?;
        Ok(response.content.into_bytes())
    }
}

pub struct ExecCtx<S: Services> {
    pub run_id: RunId,
    pub budget: BudgetImpl,
    pub services: S,
    pub trace: Box<dyn TraceSink>,
    pub cancel: CancellationToken,
    pub start_time: Instant,
    pub step_count: u32,
    pub branch_count: u32,
    pub total_tokens: u64,
    event_sequence: AtomicU64,
    latest_state: Mutex<Option<StateEnvelope>>,
}

impl<S: Services> crate::events::TraceSink for ExecCtx<S> {
    fn emit(&self, event: crate::events::ExecutionEvent) -> crate::events::EventResult {
        self.trace.emit(event)
    }
}

impl<S: Services> ExecCtx<S> {
    pub fn new(run_id: RunId, services: S) -> Self {
        Self {
            run_id,
            budget: BudgetImpl::new(),
            services,
            trace: Box::new(crate::events::NoOpTraceSink),
            cancel: CancellationToken::new(),
            start_time: Instant::now(),
            step_count: 0,
            branch_count: 0,
            total_tokens: 0,
            event_sequence: AtomicU64::new(0),
            latest_state: Mutex::new(None),
        }
    }

    pub fn next_sequence_number(&self) -> u64 {
        self.event_sequence.fetch_add(1, Ordering::SeqCst)
    }

    pub fn with_budget(mut self, budget: BudgetState) -> Self {
        self.budget = BudgetImpl::new().with_state(budget);
        self
    }

    pub fn with_trace(mut self, trace: Box<dyn TraceSink>) -> Self {
        self.trace = trace;
        self
    }

    pub fn with_cancel(mut self, cancel: CancellationToken) -> Self {
        self.cancel = cancel;
        self
    }

    pub fn with_services<S2: Services>(self, services: S2) -> ExecCtx<S2> {
        ExecCtx {
            run_id: self.run_id,
            budget: self.budget,
            services,
            trace: self.trace,
            cancel: self.cancel,
            start_time: self.start_time,
            step_count: self.step_count,
            branch_count: self.branch_count,
            total_tokens: self.total_tokens,
            event_sequence: self.event_sequence,
            latest_state: self.latest_state,
        }
    }

    pub fn inc_steps(&mut self) {
        self.step_count += 1;
    }

    pub fn inc_branches(&mut self) {
        self.branch_count += 1;
    }

    pub fn add_tokens(&mut self, tokens: u64) {
        self.total_tokens += tokens;
    }

    pub fn remember_state(&self, state: &StateEnvelope) {
        if let Ok(mut latest_state) = self.latest_state.lock() {
            *latest_state = Some(state.clone());
        }
    }

    pub fn latest_state(&self) -> Option<StateEnvelope> {
        self.latest_state
            .lock()
            .ok()
            .and_then(|state| state.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::Method::POST;
    use httpmock::MockServer;

    #[test]
    fn budget_state_default() {
        let state = BudgetState::default();
        assert!(state.max_steps.is_none());
        assert!(state.max_branches.is_none());
    }

    #[test]
    fn budget_impl_new() {
        let budget = BudgetImpl::new();
        assert!(budget.step_limit().is_none());
    }

    #[test]
    fn budget_impl_with_state() {
        let budget = BudgetImpl::new().with_state(BudgetState::new().with_max_steps(100));
        assert_eq!(budget.step_limit(), Some(100));
    }

    #[test]
    fn exec_ctx_new() {
        struct NoServices;
        impl Services for NoServices {
            type Error = std::io::Error;
            async fn call(&self, _: &str, _: &[u8]) -> Result<Vec<u8>, Self::Error> {
                Ok(vec![])
            }
        }
        let ctx = ExecCtx::new(RunId::new(), NoServices);
        assert_eq!(ctx.step_count, 0);
    }

    #[test]
    fn exec_ctx_inc_steps() {
        struct NoServices;
        impl Services for NoServices {
            type Error = std::io::Error;
            async fn call(&self, _: &str, _: &[u8]) -> Result<Vec<u8>, Self::Error> {
                Ok(vec![])
            }
        }
        let mut ctx = ExecCtx::new(RunId::new(), NoServices);
        ctx.inc_steps();
        ctx.inc_steps();
        assert_eq!(ctx.step_count, 2);
    }

    #[tokio::test]
    async fn llm_service_calls_openai_compatible_provider() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/v1/chat/completions")
                .header("Authorization", "Bearer test-key");
            then.status(200)
                .header("Content-Type", "application/json")
                .body(
                    r#"{
                        "id": "chatcmpl-123",
                        "model": "gpt-5",
                        "choices": [{
                            "message": {
                                "role": "assistant",
                                "content": "Hello from provider"
                            },
                            "finish_reason": "stop"
                        }],
                        "usage": {
                            "prompt_tokens": 10,
                            "completion_tokens": 5,
                            "total_tokens": 15
                        }
                    }"#,
                );
        });

        let service = LlmService::from_config(
            LlmServiceConfig::new()
                .provider(ProviderType::OpenAi)
                .with_api_key("test-key")
                .with_endpoint(server.url(""))
                .with_model("gpt-5"),
        )
        .unwrap();

        let response = service.call("llm", b"Say hello").await.unwrap();
        assert_eq!(String::from_utf8(response).unwrap(), "Hello from provider");
        mock.assert();
    }
}
