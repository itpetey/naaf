use std::path::PathBuf;

use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
pub struct SourceMetadata {
    pub source_type: SourceType,
    pub path: Option<PathBuf>,
    pub title: Option<String>,
    pub language: Option<String>,
    pub line_range: Option<(usize, usize)>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SourceType {
    Markdown,
    Code,
    Conversation,
    Paper,
    PlainText,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KnowledgePayload {
    pub title: String,
    pub content: String,
    pub entity_type: EntityType,
    pub repo: Option<String>,
    pub source_ids: Vec<uuid::Uuid>,
    pub related_ids: Vec<uuid::Uuid>,
    pub tags: Vec<String>,
    pub confidence: f32,
    pub source_metadata: Option<SourceMetadata>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl KnowledgePayload {
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

    pub fn with_repo(mut self, repo: impl Into<String>) -> Self {
        self.repo = Some(repo.into());
        self
    }

    pub fn with_source(mut self, source_id: uuid::Uuid) -> Self {
        self.source_ids.push(source_id);
        self
    }

    pub fn with_related(mut self, related_id: uuid::Uuid) -> Self {
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

    pub fn with_source_metadata(mut self, metadata: SourceMetadata) -> Self {
        self.source_metadata = Some(metadata);
        self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: uuid::Uuid,
    pub score: f32,
    pub payload: KnowledgePayload,
}
