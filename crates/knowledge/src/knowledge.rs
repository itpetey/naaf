use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// One knowledge entry stored or exchanged by the knowledge layer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    /// Stable identifier for the entry.
    pub id: Uuid,
    /// Human-readable title.
    pub title: String,
    /// Main body content.
    pub content: String,
    /// Semantic classification.
    pub entity_type: EntityType,
    /// Source entries that underpin this entry.
    pub source_ids: Vec<Uuid>,
    /// Related knowledge entries.
    pub related_ids: Vec<Uuid>,
    /// Tags used for grouping or retrieval.
    pub tags: Vec<String>,
    /// Confidence score assigned by the producer.
    pub confidence: f32,
    /// Creation timestamp in UTC.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp in UTC.
    pub updated_at: DateTime<Utc>,
}

/// One issue reported by the collection linter.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LintIssue {
    /// Category of issue that was detected.
    pub issue_type: LintIssueType,
    /// Human-readable explanation of the issue.
    pub description: String,
    /// Identifiers of entries involved in the issue.
    pub entry_ids: Vec<Uuid>,
    /// Optional remediation suggestion.
    pub suggestion: Option<String>,
}

/// Aggregate report returned by collection linting.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LintReport {
    /// Issues found during linting.
    pub issues: Vec<LintIssue>,
    /// Number of entries inspected.
    pub entries_scanned: usize,
}

/// Semantic type assigned to a knowledge entry.
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
    /// Raw source material stored directly.
    Source,
}

/// Summary of one ingest operation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IngestReport {
    /// Source point identifiers created during ingestion.
    pub source_ids: Vec<Uuid>,
    /// Higher-level knowledge identifiers created during ingestion.
    pub knowledge_ids: Vec<Uuid>,
    /// Number of source chunks written.
    pub chunks_count: usize,
    /// Number of derived knowledge entries written.
    pub entries_count: usize,
}

/// Category of lint issue detected in the collection.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum LintIssueType {
    /// Two or more entries disagree.
    Contradiction,
    /// An entry lacks references to the rest of the graph.
    Orphan,
    /// An entry appears out of date.
    Stale,
    /// An expected cross-reference is missing.
    MissingCrossReference,
    /// Supporting data is incomplete or low quality.
    DataGap,
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

impl KnowledgeEntry {
    /// Creates a new entry with generated identifiers and timestamps.
    pub fn new(title: String, content: String, entity_type: EntityType) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            title,
            content,
            entity_type,
            source_ids: Vec::new(),
            related_ids: Vec::new(),
            tags: Vec::new(),
            confidence: 1.0,
            created_at: now,
            updated_at: now,
        }
    }

    /// Appends one source identifier.
    pub fn with_source(mut self, source_id: Uuid) -> Self {
        self.source_ids.push(source_id);
        self
    }

    /// Appends one related entry identifier.
    pub fn with_related(mut self, related_id: Uuid) -> Self {
        self.related_ids.push(related_id);
        self
    }

    /// Replaces the tag list.
    pub fn with_tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags = tags.into_iter().map(|t| t.into()).collect();
        self
    }

    /// Sets the confidence score.
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence;
        self
    }
}
