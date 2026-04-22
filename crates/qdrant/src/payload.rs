use std::fmt;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Payload stored in each Qdrant point.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KnowledgePayload {
    /// Display title for the entry.
    pub title: String,
    /// Main text content used for retrieval.
    pub content: String,
    /// Semantic classification of the entry.
    pub entity_type: EntityType,
    /// Optional repository scope for multi-repo collections.
    pub repo: Option<String>,
    /// Source entries that underpin this entry.
    pub source_ids: Vec<uuid::Uuid>,
    /// Related knowledge entries.
    pub related_ids: Vec<uuid::Uuid>,
    /// Search and grouping tags.
    pub tags: Vec<String>,
    /// Confidence score assigned by the producer.
    pub confidence: f32,
    /// Optional metadata about the original source.
    pub source_metadata: Option<SourceMetadata>,
    /// Creation timestamp in UTC.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp in UTC.
    pub updated_at: DateTime<Utc>,
}

/// Metadata describing where a knowledge payload originated.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceMetadata {
    /// High-level source classification.
    pub source_type: SourceType,
    /// Path to the originating file, when known.
    pub path: Option<PathBuf>,
    /// Human-readable source title.
    pub title: Option<String>,
    /// Programming or markup language associated with the source.
    pub language: Option<String>,
    /// Recorded span within the source material.
    pub line_range: Option<(usize, usize)>,
}

/// One scored search hit returned from Qdrant.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchResult {
    /// Identifier of the matched point.
    pub id: uuid::Uuid,
    /// Similarity score returned by Qdrant.
    pub score: f32,
    /// Stored payload for the matched point.
    pub payload: KnowledgePayload,
}

/// Semantic type assigned to a stored knowledge entry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum EntityType {
    /// A general concept or topic.
    Concept,
    /// A concrete entity such as a person, system, or component.
    Entity,
    /// A summary distilled from source material.
    Summary,
    /// A comparison between multiple entities or concepts.
    Comparison,
    /// An analytical conclusion or interpretation.
    Analysis,
    /// A question-and-answer style entry.
    QuestionAnswer,
    /// Raw source material chunked directly from input content.
    Source,
}

/// Type of source content stored in the vector database.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SourceType {
    /// Markdown or other heading-oriented prose.
    Markdown,
    /// Source code.
    Code,
    /// Conversation transcript data.
    Conversation,
    /// PDF or paper-like document content.
    Paper,
    /// Plain text content without richer structure.
    PlainText,
}

impl fmt::Display for EntityType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EntityType::Concept => write!(f, "concept"),
            EntityType::Entity => write!(f, "entity"),
            EntityType::Summary => write!(f, "summary"),
            EntityType::Comparison => write!(f, "comparison"),
            EntityType::Analysis => write!(f, "analysis"),
            EntityType::QuestionAnswer => write!(f, "qa"),
            EntityType::Source => write!(f, "source"),
        }
    }
}

impl KnowledgePayload {
    /// Creates a new payload with default metadata and timestamps.
    pub fn new(title: String, content: String, entity_type: EntityType) -> Self {
        let now = Utc::now();
        Self {
            title,
            content,
            entity_type,
            repo: None,
            source_ids: Vec::new(),
            related_ids: Vec::new(),
            tags: Vec::new(),
            confidence: 1.0,
            source_metadata: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Sets the repository identifier for this payload.
    pub fn with_repo(mut self, repo: impl Into<String>) -> Self {
        self.repo = Some(repo.into());
        self
    }

    /// Appends one source identifier.
    pub fn with_source(mut self, source_id: uuid::Uuid) -> Self {
        self.source_ids.push(source_id);
        self
    }

    /// Appends one related knowledge identifier.
    pub fn with_related(mut self, related_id: uuid::Uuid) -> Self {
        self.related_ids.push(related_id);
        self
    }

    /// Replaces the tag list.
    pub fn with_tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags = tags.into_iter().map(|t| t.into()).collect();
        self
    }

    /// Sets the payload confidence score.
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence;
        self
    }

    /// Attaches source metadata to the payload.
    pub fn with_source_metadata(mut self, metadata: SourceMetadata) -> Self {
        self.source_metadata = Some(metadata);
        self
    }
}
