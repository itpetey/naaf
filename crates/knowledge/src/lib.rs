//! Knowledge orchestration for `naaf`.
#![warn(missing_docs)]
//!
//! `naaf-knowledge` implements the Karpathy-style LLM Wiki pattern using Qdrant
//! as the persistent knowledge store. Three core operations:
//!
//! - **Ingest**: Chunk source content, embed it, upsert into Qdrant, and
//!   optionally run LLM extraction to produce concept/entity/summary entries.
//! - **Query**: Search Qdrant for relevant entries, synthesise an answer with
//!   an LLM, and optionally re-ingest the answer as a new knowledge entry.
//! - **Lint**: Scan the knowledge base for contradictions, orphans, stale
//!   entries, and missing cross-references.
//!
//! All operations compose through `naaf_core`'s `Task`, `Check`, `Materialiser`,
//! and `Step` types.

/// Error and result types for knowledge workflows.
pub mod error;
/// Ingestion helpers for files, directories, and in-memory content.
pub mod ingest;
/// Core knowledge-domain types such as entries and lint reports.
pub mod knowledge;
/// Collection-wide linting helpers.
pub mod lint;
/// Retrieval helpers built on top of Qdrant search.
pub mod query;
/// Source description types used during ingestion.
pub mod source;
/// LLM tool wrapper for querying and linting the knowledge base.
pub mod tool;

pub use error::{KnowledgeError, Result};
pub use ingest::DirectoryIngestReport;
pub use knowledge::{
    EntityType, IngestReport, KnowledgeEntry, LintIssue, LintIssueType, LintReport,
};
pub use source::{SourceInfo, SourceType};
pub use tool::KnowledgeTool;
