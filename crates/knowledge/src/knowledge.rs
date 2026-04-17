use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum EntityType {
    Concept,
    Entity,
    Summary,
    Comparison,
    Analysis,
    QuestionAnswer,
    Source,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub entity_type: EntityType,
    pub source_ids: Vec<Uuid>,
    pub related_ids: Vec<Uuid>,
    pub tags: Vec<String>,
    pub confidence: f32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl KnowledgeEntry {
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

    pub fn with_source(mut self, source_id: Uuid) -> Self {
        self.source_ids.push(source_id);
        self
    }

    pub fn with_related(mut self, related_id: Uuid) -> Self {
        self.related_ids.push(related_id);
        self
    }

    pub fn with_tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags = tags.into_iter().map(|t| t.into()).collect();
        self
    }

    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence;
        self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IngestReport {
    pub source_ids: Vec<Uuid>,
    pub knowledge_ids: Vec<Uuid>,
    pub chunks_count: usize,
    pub entries_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LintIssue {
    pub issue_type: LintIssueType,
    pub description: String,
    pub entry_ids: Vec<Uuid>,
    pub suggestion: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum LintIssueType {
    Contradiction,
    Orphan,
    Stale,
    MissingCrossReference,
    DataGap,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LintReport {
    pub issues: Vec<LintIssue>,
    pub entries_scanned: usize,
}
