//! Knowledge orchestration for `naaf`.
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

pub mod error;
pub mod ingest;
pub mod knowledge;
pub mod lint;
pub mod query;
pub mod source;
pub mod tool;

pub use error::{KnowledgeError, Result};
pub use ingest::DirectoryIngestReport;
pub use knowledge::{
    EntityType, IngestReport, KnowledgeEntry, LintIssue, LintIssueType, LintReport,
};
pub use source::{SourceInfo, SourceType};
pub use tool::KnowledgeTool;
