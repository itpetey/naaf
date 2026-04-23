use naaf_llm::{CompletionRequest, Message, RegisterToolError, ToolRegistry};
use naaf_qdrant::{Embedder, QdrantClient};

use crate::{
    KnowledgeError, KnowledgeGroup, KnowledgeLintTool, KnowledgePromptConfig, KnowledgeSearchTool,
    augment_system_prompt,
};

/// Configures how knowledge groups are exposed to LLM requests.
#[derive(Clone, Debug, PartialEq)]
pub struct KnowledgeLlmConfig {
    /// Prompt-shaping options used for the generated system prompt.
    pub prompt: KnowledgePromptConfig,
    /// Whether the lint tool should be registered alongside search.
    pub include_lint_tool: bool,
    /// Default merged result count exposed through `knowledge_search`.
    pub search_top_k: usize,
    /// Default similarity floor exposed through `knowledge_search`.
    pub search_min_score: f32,
    /// Optional default repository filter applied to searches.
    pub repo: Option<String>,
}

/// One knowledge-group target together with the client used to query it.
#[derive(Clone)]
pub struct KnowledgeLlmTarget {
    /// Metadata describing the exposed knowledge collection.
    pub group: KnowledgeGroup,
    /// Qdrant client scoped to the target collection.
    pub client: QdrantClient,
}

/// Reusable LLM session state derived from selected knowledge groups.
#[derive(Clone)]
pub struct KnowledgeLlmSession<R> {
    system_prompt: String,
    tools: ToolRegistry<R, KnowledgeError>,
}

/// Builder that wires knowledge prompt augmentation and tool registration together.
pub struct KnowledgeLlmSessionBuilder<R> {
    system_prompt: String,
    targets: Vec<KnowledgeLlmTarget>,
    embedder: Box<dyn Embedder>,
    config: KnowledgeLlmConfig,
    _marker: std::marker::PhantomData<R>,
}

impl Default for KnowledgeLlmConfig {
    fn default() -> Self {
        Self {
            prompt: KnowledgePromptConfig::default(),
            include_lint_tool: false,
            search_top_k: 5,
            search_min_score: 0.7,
            repo: None,
        }
    }
}

impl KnowledgeLlmTarget {
    /// Creates one knowledge target from a group and its client.
    pub fn new(group: KnowledgeGroup, client: QdrantClient) -> Self {
        Self { group, client }
    }
}

impl<R> KnowledgeLlmSession<R> {
    /// Returns the generated system prompt.
    pub fn system_prompt(&self) -> &str {
        &self.system_prompt
    }

    /// Returns the configured tool registry.
    pub fn tools(&self) -> &ToolRegistry<R, KnowledgeError> {
        &self.tools
    }

    /// Builds a completion request by prepending the generated system prompt.
    pub fn request(&self, model: impl Into<String>, messages: Vec<Message>) -> CompletionRequest {
        let mut request_messages = Vec::new();

        if !self.system_prompt.is_empty() {
            request_messages.push(Message::system(self.system_prompt.clone()));
        }

        request_messages.extend(messages);
        CompletionRequest::new(model, request_messages)
    }

    /// Builds a completion request with a single user message.
    pub fn request_with_user_message(
        &self,
        model: impl Into<String>,
        user_content: impl Into<String>,
    ) -> CompletionRequest {
        self.request(model, vec![Message::user(user_content)])
    }

    /// Consumes the session into its generated system prompt and tool registry.
    pub fn into_parts(self) -> (String, ToolRegistry<R, KnowledgeError>) {
        (self.system_prompt, self.tools)
    }
}

impl<R> KnowledgeLlmSessionBuilder<R> {
    /// Creates a builder using the supplied embedder for search-tool queries.
    pub fn new(embedder: Box<dyn Embedder>) -> Self {
        Self {
            system_prompt: String::new(),
            targets: Vec::new(),
            embedder,
            config: KnowledgeLlmConfig::default(),
            _marker: std::marker::PhantomData,
        }
    }

    /// Replaces the base system prompt.
    pub fn with_system_prompt(mut self, system_prompt: impl Into<String>) -> Self {
        self.system_prompt = system_prompt.into();
        self
    }

    /// Adds one configured knowledge target.
    pub fn with_target(mut self, target: KnowledgeLlmTarget) -> Self {
        self.targets.push(target);
        self
    }

    /// Adds one configured knowledge group and its client.
    pub fn with_group(mut self, group: KnowledgeGroup, client: QdrantClient) -> Self {
        self.targets.push(KnowledgeLlmTarget::new(group, client));
        self
    }

    /// Replaces the full LLM integration config.
    pub fn with_config(mut self, config: KnowledgeLlmConfig) -> Self {
        self.config = config;
        self
    }

    /// Replaces just the prompt configuration.
    pub fn with_prompt_config(mut self, prompt: KnowledgePromptConfig) -> Self {
        self.config.prompt = prompt;
        self
    }

    /// Controls whether the lint tool is registered.
    pub fn with_lint_tool(mut self, include_lint_tool: bool) -> Self {
        self.config.include_lint_tool = include_lint_tool;
        self
    }

    /// Replaces the default search settings used by the search tool.
    pub fn with_search_defaults(mut self, top_k: usize, min_score: f32) -> Self {
        self.config.search_top_k = top_k;
        self.config.search_min_score = min_score;
        self
    }

    /// Applies a default repository filter to search requests.
    pub fn with_repo(mut self, repo: impl Into<String>) -> Self {
        self.config.repo = Some(repo.into());
        self
    }

    /// Builds the reusable LLM session.
    pub fn build(self) -> Result<KnowledgeLlmSession<R>, RegisterToolError>
    where
        R: 'static,
    {
        let groups = self
            .targets
            .iter()
            .map(|target| target.group.clone())
            .collect::<Vec<_>>();
        let mut tools = ToolRegistry::new();
        let system_prompt = if groups.is_empty() {
            self.system_prompt
        } else {
            let mut prompt = self.config.prompt.clone();
            prompt.tool_name = "knowledge_search".to_string();
            augment_system_prompt(&self.system_prompt, &groups, &prompt)
        };

        if !groups.is_empty() {
            let mut search_tool = KnowledgeSearchTool::new(
                self.embedder,
                self.config.search_top_k,
                self.config.search_min_score,
            );

            if let Some(repo) = self.config.repo {
                search_tool = search_tool.with_repo(repo);
            }

            for target in &self.targets {
                search_tool = search_tool.with_group(target.group.clone(), target.client.clone());
            }

            tools.register(search_tool)?;

            if self.config.include_lint_tool {
                let mut lint_tool = KnowledgeLintTool::new();
                for target in &self.targets {
                    lint_tool = lint_tool.with_group(target.group.clone(), target.client.clone());
                }
                tools.register(lint_tool)?;
            }
        }

        Ok(KnowledgeLlmSession {
            system_prompt,
            tools,
        })
    }
}

#[cfg(test)]
mod tests {
    use futures::future::LocalBoxFuture;
    use naaf_llm::Message;
    use naaf_qdrant::{Embedder, QdrantClient, QdrantError};

    use super::KnowledgeLlmSessionBuilder;
    use crate::KnowledgeGroup;

    struct StubEmbedder;

    impl Embedder for StubEmbedder {
        fn embed<'a>(
            &'a self,
            texts: Vec<String>,
        ) -> LocalBoxFuture<'a, Result<Vec<Vec<f32>>, QdrantError>> {
            Box::pin(async move { Ok(texts.into_iter().map(|_| vec![1.0, 2.0]).collect()) })
        }

        fn dimension(&self) -> usize {
            2
        }
    }

    #[test]
    fn session_builder_wires_system_prompt_and_tools() {
        let client = QdrantClient::from_url("http://localhost:6333", Option::<String>::None)
            .expect("client should build")
            .with_collection("docs");
        let session = KnowledgeLlmSessionBuilder::<()>::new(Box::new(StubEmbedder))
            .with_system_prompt("You are helpful.")
            .with_group(
                KnowledgeGroup::new("docs", "Documentation", "Product documentation"),
                client,
            )
            .with_lint_tool(true)
            .build()
            .expect("session should build");

        assert!(session.system_prompt().contains("You are helpful."));
        assert!(
            session
                .system_prompt()
                .contains("Use the `knowledge_search` tool")
        );

        let tool_names = session
            .tools()
            .specs()
            .into_iter()
            .map(|spec| spec.name)
            .collect::<Vec<_>>();
        assert_eq!(tool_names, vec!["knowledge_lint", "knowledge_search"]);
    }

    #[test]
    fn session_request_prepends_generated_system_prompt() {
        let client = QdrantClient::from_url("http://localhost:6333", Option::<String>::None)
            .expect("client should build")
            .with_collection("docs");
        let session = KnowledgeLlmSessionBuilder::<()>::new(Box::new(StubEmbedder))
            .with_system_prompt("You are helpful.")
            .with_group(
                KnowledgeGroup::new("docs", "Documentation", "Product documentation"),
                client,
            )
            .build()
            .expect("session should build");
        let request = session.request_with_user_message("gpt-4o", "What is naaf-core?");

        assert_eq!(request.model, "gpt-4o");
        assert_eq!(request.messages.len(), 2);
        match &request.messages[0] {
            Message::System { content } => {
                assert!(content.contains("Collection: docs"));
            }
            message => panic!("expected system message, got {message:?}"),
        }
    }

    #[test]
    fn session_builder_keeps_plain_system_prompt_without_groups() {
        let session = KnowledgeLlmSessionBuilder::<()>::new(Box::new(StubEmbedder))
            .with_system_prompt("You are helpful.")
            .build()
            .expect("session should build");

        assert_eq!(session.system_prompt(), "You are helpful.");
        assert!(session.tools().is_empty());
    }
}
