use naaf_qdrant::QdrantClient;

use crate::error::KnowledgeError;
use crate::knowledge::{LintIssue, LintIssueType, LintReport};

/// Scans the configured collection for basic graph and metadata issues.
pub async fn lint_collection(client: &QdrantClient) -> Result<LintReport, KnowledgeError> {
    let all_entries = client
        .scroll(100, None)
        .await
        .map_err(KnowledgeError::Qdrant)?;

    let mut issues = Vec::new();

    for entry in &all_entries {
        if entry.payload.related_ids.is_empty()
            && !matches!(entry.payload.entity_type, naaf_qdrant::EntityType::Source)
        {
            issues.push(LintIssue {
                issue_type: LintIssueType::Orphan,
                description: format!(
                    "'{}' ({:?}) has no inbound references",
                    entry.payload.title, entry.payload.entity_type
                ),
                entry_ids: vec![entry.id],
                suggestion: Some(
                    "Consider linking this entry to related concepts or sources".to_string(),
                ),
            });
        }

        if !entry.payload.tags.is_empty() {
            let lowercase_tags: Vec<String> = entry
                .payload
                .tags
                .iter()
                .map(|t| t.to_lowercase())
                .collect();
            let unique_count = lowercase_tags
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len();
            if unique_count < entry.payload.tags.len() {
                issues.push(LintIssue {
                    issue_type: LintIssueType::DataGap,
                    description: format!(
                        "'{}' has duplicate tags: {:?}",
                        entry.payload.title, entry.payload.tags
                    ),
                    entry_ids: vec![entry.id],
                    suggestion: Some("Remove duplicate tags".to_string()),
                });
            }
        }
    }

    Ok(LintReport {
        issues,
        entries_scanned: all_entries.len(),
    })
}
