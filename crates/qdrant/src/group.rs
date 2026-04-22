use std::{future::Future, pin::Pin};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Result type returned by knowledge-group stores.
pub type KnowledgeGroupStoreResult<T> =
    Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

/// Boxed future type used by knowledge-group stores.
pub type KnowledgeGroupStoreFuture<T> =
    Pin<Box<dyn Future<Output = KnowledgeGroupStoreResult<T>> + Send>>;

/// Rich metadata describing one Qdrant-backed knowledge group.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KnowledgeGroup {
    /// Canonical Qdrant collection identifier.
    pub collection: String,
    /// Human-friendly display name.
    pub name: String,
    /// High-level description used to explain the collection's purpose.
    pub description: String,
    /// Search and grouping labels associated with the collection.
    pub tags: Vec<String>,
    /// Query guidance that can be supplied to an LLM.
    pub query_hints: Vec<String>,
    /// Additional free-form metadata for application-specific context.
    pub metadata: Map<String, Value>,
    /// Creation timestamp in UTC.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp in UTC.
    pub updated_at: DateTime<Utc>,
}

/// Pluggable store for persisting knowledge-group metadata.
pub trait KnowledgeGroupStore: Send + Sync + 'static {
    /// Creates or updates a stored knowledge group.
    fn upsert_group(&self, group: &KnowledgeGroup) -> KnowledgeGroupStoreFuture<()>;

    /// Loads one knowledge group by collection name.
    fn load_group(&self, collection: &str) -> KnowledgeGroupStoreFuture<Option<KnowledgeGroup>>;

    /// Lists all stored knowledge groups.
    fn list_groups(&self) -> KnowledgeGroupStoreFuture<Vec<KnowledgeGroup>>;

    /// Deletes one knowledge group by collection name.
    fn delete_group(&self, collection: &str) -> KnowledgeGroupStoreFuture<()>;
}

impl KnowledgeGroup {
    /// Creates a new knowledge group with generated timestamps.
    pub fn new(
        collection: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            collection: collection.into(),
            name: name.into(),
            description: description.into(),
            tags: Vec::new(),
            query_hints: Vec::new(),
            metadata: Map::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Replaces the tag list.
    pub fn with_tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags = tags.into_iter().map(|tag| tag.into()).collect();
        self
    }

    /// Replaces the query-hint list.
    pub fn with_query_hints(
        mut self,
        query_hints: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.query_hints = query_hints.into_iter().map(|hint| hint.into()).collect();
        self
    }

    /// Replaces the free-form metadata map.
    pub fn with_metadata(mut self, metadata: Map<String, Value>) -> Self {
        self.metadata = metadata;
        self
    }

    /// Adds or replaces one free-form metadata field.
    pub fn with_metadata_field(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Returns a copy ready to persist, preserving the original creation time when updating.
    pub fn prepare_for_upsert(&self, existing: Option<&KnowledgeGroup>) -> Self {
        let mut group = self.clone();
        if let Some(existing) = existing {
            group.created_at = existing.created_at;
        }
        group.updated_at = Utc::now();
        group
    }
}

/// Formats knowledge groups into deterministic prompt context for LLMs.
pub fn format_knowledge_groups_for_prompt(groups: &[KnowledgeGroup]) -> String {
    let mut sorted_groups = groups.to_vec();
    sorted_groups.sort_by(|left, right| left.collection.cmp(&right.collection));

    sorted_groups
        .into_iter()
        .map(|group| {
            let mut lines = vec![
                format!("Collection: {}", group.collection),
                format!("Name: {}", group.name),
                format!("Description: {}", group.description),
            ];

            if !group.tags.is_empty() {
                lines.push(format!("Tags: {}", group.tags.join(", ")));
            }

            if !group.query_hints.is_empty() {
                lines.push(format!("Query hints: {}", group.query_hints.join(" | ")));
            }

            if !group.metadata.is_empty() {
                let mut metadata = group.metadata.into_iter().collect::<Vec<_>>();
                metadata.sort_by(|left, right| left.0.cmp(&right.0));
                let rendered = metadata
                    .into_iter()
                    .map(|(key, value)| format!("{key}={}", value_to_inline_text(&value)))
                    .collect::<Vec<_>>()
                    .join(", ");
                lines.push(format!("Extra metadata: {rendered}"));
            }

            lines.join("\n")
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn value_to_inline_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        _ => serde_json::to_string(value).unwrap_or_else(|_| "null".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use serde_json::json;

    use super::{KnowledgeGroup, format_knowledge_groups_for_prompt};

    #[test]
    fn knowledge_group_round_trips_through_json() {
        let group = KnowledgeGroup::new("docs", "Documentation", "Product documentation")
            .with_tags(["rust", "api"])
            .with_query_hints(["Prefer API references", "Use examples when present"])
            .with_metadata_field("owner", json!("docs-team"))
            .with_metadata_field("priority", json!(3));

        let encoded = serde_json::to_string(&group).expect("group should serialise");
        let decoded: KnowledgeGroup =
            serde_json::from_str(&encoded).expect("group should deserialise");

        assert_eq!(decoded.collection, "docs");
        assert_eq!(decoded.name, "Documentation");
        assert_eq!(decoded.tags, vec!["rust", "api"]);
        assert_eq!(decoded.metadata["owner"], json!("docs-team"));
        assert_eq!(decoded.metadata["priority"], json!(3));
    }

    #[test]
    fn prepare_for_upsert_preserves_created_at_and_refreshes_updated_at() {
        let existing = KnowledgeGroup {
            collection: "docs".to_string(),
            name: "Old".to_string(),
            description: "Old description".to_string(),
            tags: Vec::new(),
            query_hints: Vec::new(),
            metadata: serde_json::Map::new(),
            created_at: Utc.with_ymd_and_hms(2024, 1, 2, 3, 4, 5).single().unwrap(),
            updated_at: Utc.with_ymd_and_hms(2024, 1, 3, 3, 4, 5).single().unwrap(),
        };
        let incoming = KnowledgeGroup {
            collection: "docs".to_string(),
            name: "New".to_string(),
            description: "New description".to_string(),
            tags: vec!["updated".to_string()],
            query_hints: Vec::new(),
            metadata: serde_json::Map::new(),
            created_at: Utc.with_ymd_and_hms(2025, 1, 2, 3, 4, 5).single().unwrap(),
            updated_at: Utc.with_ymd_and_hms(2025, 1, 3, 3, 4, 5).single().unwrap(),
        };

        let before = Utc::now();
        let prepared = incoming.prepare_for_upsert(Some(&existing));
        let after = Utc::now();

        assert_eq!(prepared.created_at, existing.created_at);
        assert!(prepared.updated_at >= before);
        assert!(prepared.updated_at <= after);
        assert_eq!(prepared.name, "New");
    }

    #[test]
    fn prompt_formatting_is_deterministic() {
        let right = KnowledgeGroup::new("zeta", "Zeta", "Tail group")
            .with_tags(["late"])
            .with_metadata_field("owner", json!("team-z"));
        let left = KnowledgeGroup::new("alpha", "Alpha", "First group")
            .with_query_hints(["Start with API docs"])
            .with_metadata_field("priority", json!(1));

        let rendered = format_knowledge_groups_for_prompt(&[right, left]);

        assert!(rendered.starts_with("Collection: alpha"));
        assert!(rendered.contains("Query hints: Start with API docs"));
        assert!(rendered.contains("Extra metadata: priority=1"));
        assert!(rendered.contains("Collection: zeta"));
    }
}
