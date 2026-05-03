//! Qdrant vector database integration for `naaf`.
#![warn(missing_docs)]
//!
//! `naaf-qdrant` provides typed adapters for Qdrant operations — upsert, search,
//! and similarity check — that plug into `naaf_core`'s `Task`, `Materialiser`,
//! and `Check` traits, and a `Tool` implementation for LLM tool calling.
//!
//! # Architecture
//!
//! The crate follows the same "shared agent, projected roles" pattern as `naaf-llm`:
//!
//! - **`QdrantAgent`** holds a `QdrantClient` and an `Embedder`, and provides
//!   high-level operations like `search()` and `upsert_chunks()`.
//! - **`QdrantSearch`** implements `Task` — query text in, search results out.
//! - **`QdrantUpsert`** implements `Materialiser` — knowledge payloads in, IDs out.
//! - **`QdrantSimilarityCheck`** implements `Check` — query text in, near-duplicate findings out.
//! - **`QdrantSearchTool`** implements naaf's `Tool` trait so LLMs can call
//!   `knowledge_search` during tool-using workflows.
//!
//! # Embedding
//!
//! The `Embedder` trait abstracts over embedding providers. Enable the `openai`
//! feature (default) for `OpenAiEmbedder`, or bring your own implementation.
//!
//! # Chunking
//!
//! Four built-in chunkers handle different content types:
//! - `MarkdownChunker` — splits by headings with overlap
//! - `CodeChunker` — splits by function/class boundaries
//! - `ConversationChunker` — splits JSON conversation transcripts by message
//! - `PdfChunker` — extracts text from PDFs then delegates to `MarkdownChunker`
//!
//! `ContentChunker` auto-detects the right chunker from a file path.

#[cfg(feature = "openai")]
pub use embedder::openai::OpenAiEmbedder;
pub use chunker::{
    Chunk, ChunkMetadata, Chunker, CodeChunker, ContentChunker, ConversationChunker,
    MarkdownChunker, PdfChunker, SourceInfo,
};
pub use client::{PointData, QdrantAgent, QdrantClient};
pub use embedder::Embedder;
pub use error::{QdrantError, Result};
pub use payload::{EntityType, KnowledgePayload, SearchResult, SourceMetadata, SourceType};
pub use task::{QdrantSearch, QdrantSimilarityCheck, QdrantUpsert};
pub use tool::QdrantSearchTool;

/// Content chunking primitives used before embedding and upsert.
pub mod chunker;
/// Conversation transcript types consumed by the conversation chunker.
pub mod conversation;
/// Embedding abstractions and provider implementations.
pub mod embedder;
/// Error and result types for this crate.
pub mod error;
/// LLM tool adapters backed by Qdrant search.
pub mod tool;
/// Low-level Qdrant client wrappers and the high-level shared agent.
pub mod client;
/// Search payload and metadata types stored in Qdrant.
pub mod payload;
/// `naaf-core` task, materialiser, and check adapters built on Qdrant.
pub mod task;
