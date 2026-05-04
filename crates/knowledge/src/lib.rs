//! Knowledge orchestration for `naaf`.
#![warn(missing_docs)]
//!
//! `naaf-knowledge` implements the Karpathy-style LLM Wiki pattern using Qdrant
//! as the persistent knowledge store. Four core operations:
//!
//! - **Ingest**: Chunk source content, embed it, upsert into Qdrant, and
//!   optionally run LLM extraction to produce concept/entity/summary entries.
//! - **Query**: Search Qdrant for relevant entries, synthesise an answer with
//!   an LLM, and optionally re-ingest the answer as a new knowledge entry.
//! - **Lint**: Scan the knowledge base for contradictions, orphans, stale
//!   entries, and missing cross-references.
//! - **Knowledge Groups**: Describe Qdrant collections with domain-level
//!   metadata and persist that metadata independently from the vector store.
//!
//! All operations compose through `naaf_core`'s `Task`, `Check`, `Materialiser`,
//! and `Step` types.
//!
//! # Recommended LLM Integration
//!
//! The idiomatic way to expose knowledge to an LLM is to:
//!
//! 1. Select the `KnowledgeGroup`s your application wants to expose.
//! 2. Build a [`KnowledgeLlmSession`] with [`KnowledgeLlmSessionBuilder`].
//! 3. Reuse the generated system prompt and tool registry across requests.
//!
//! ```ignore
//! use naaf_knowledge::{KnowledgeGroup, KnowledgeLlmSessionBuilder};
//! use naaf_llm::{Executor, OpenAiClient, OpenAiConfig};
//!
//! let client = naaf_qdrant::QdrantClient::from_url(
//!     "http://localhost:6333",
//!     Option::<String>::None,
//! )?
//! .with_collection("docs");
//! let embedder = naaf_qdrant::OpenAiEmbedder::new(std::env::var("OPENAI_API_KEY")?);
//!
//! let knowledge = KnowledgeLlmSessionBuilder::new(Box::new(embedder))
//!     .with_system_prompt("You are a helpful assistant for the workspace.")
//!     .with_group(
//!         KnowledgeGroup::new("docs", "Documentation", "Product and API documentation"),
//!         client,
//!     )
//!     .with_search_defaults(5, 0.7)
//!     .build()?;
//!
//! let llm_client = OpenAiClient::new(OpenAiConfig::new(std::env::var("OPENAI_API_KEY")?));
//! let executor = Executor::with_tools(llm_client, knowledge.tools().clone());
//! let request = knowledge.request_with_user_message("gpt-4o", "How do steps retry?");
//! let outcome = executor.execute(&(), request).await?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Lower-level building blocks remain available when you need more control:
//! [`KnowledgeSearchTool`], [`KnowledgeLintTool`], [`KnowledgePromptConfig`],
//! [`format_knowledge_prompt_block`], and [`augment_system_prompt`].

pub use error::{KnowledgeError, Result};
pub use group::{
    KnowledgeGroup, KnowledgeGroupStore, KnowledgeGroupStoreFuture, KnowledgeGroupStoreResult,
    KnowledgePromptConfig, augment_system_prompt, format_knowledge_groups_for_prompt,
    format_knowledge_prompt_block,
};
pub use ingest::DirectoryIngestReport;
pub use knowledge::{
    EntityType, IngestReport, KnowledgeEntry, LintIssue, LintIssueType, LintReport,
};
pub use llm::{
    KnowledgeLlmConfig, KnowledgeLlmSession, KnowledgeLlmSessionBuilder, KnowledgeLlmTarget,
};
pub use source::{SourceInfo, SourceType};
pub use tool::{KnowledgeLintTool, KnowledgeSearchTool};

/// Error and result types for knowledge workflows.
pub mod error;
/// Knowledge-group metadata types and persistence traits.
pub mod group;
/// Ingestion helpers for files, directories, and in-memory content.
pub mod ingest;
/// Core knowledge-domain types such as entries and lint reports.
pub mod knowledge;
/// Collection-wide linting helpers.
pub mod lint;
/// LLM session helpers that combine prompt augmentation and tool registration.
pub mod llm;
/// Retrieval helpers built on top of Qdrant search.
pub mod query;
/// Source description types used during ingestion.
pub mod source;
/// LLM tools for searching and linting the knowledge base.
pub mod tool;
