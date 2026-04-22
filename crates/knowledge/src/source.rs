use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Metadata describing a source to ingest.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceInfo {
    /// High-level source classification.
    pub source_type: SourceType,
    /// Path to the source on disk, when applicable.
    pub path: Option<PathBuf>,
    /// Human-readable source title.
    pub title: Option<String>,
    /// Programming or markup language associated with the source.
    pub language: Option<String>,
    /// In-memory source content, when not reading from disk.
    pub content: Option<String>,
}

/// Type of source material that may be ingested into the knowledge base.
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
    /// Directory input that should be walked recursively.
    Directory,
}

impl SourceInfo {
    /// Builds source metadata from a file path.
    pub fn from_path(path: &std::path::Path) -> Result<Self, crate::error::KnowledgeError> {
        let source_type = detect_source_type(path);
        let language = detect_language(path);
        let title = path.file_stem().and_then(|s| s.to_str()).map(String::from);
        Ok(Self {
            source_type,
            path: Some(path.to_path_buf()),
            title,
            language,
            content: None,
        })
    }

    /// Creates metadata for in-memory markdown content.
    pub fn markdown(content: &str, title: Option<String>) -> Self {
        Self {
            source_type: SourceType::Markdown,
            path: None,
            title,
            language: None,
            content: Some(content.to_string()),
        }
    }

    /// Creates metadata for an in-memory conversation transcript.
    pub fn conversation(content: &str, title: Option<String>) -> Self {
        Self {
            source_type: SourceType::Conversation,
            path: None,
            title,
            language: None,
            content: Some(content.to_string()),
        }
    }
}

fn detect_language(path: &std::path::Path) -> Option<String> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("rs") => Some("rust".to_string()),
        Some("py") => Some("python".to_string()),
        Some("ts") => Some("typescript".to_string()),
        Some("js") => Some("javascript".to_string()),
        Some("go") => Some("go".to_string()),
        Some("java") => Some("java".to_string()),
        _ => None,
    }
}

fn detect_source_type(path: &std::path::Path) -> SourceType {
    match path.extension().and_then(|e| e.to_str()) {
        Some("pdf") => SourceType::Paper,
        Some("md" | "txt") => SourceType::Markdown,
        Some("json") => SourceType::Conversation,
        Some("rs" | "py" | "ts" | "js" | "go" | "java" | "c" | "cpp" | "h") => SourceType::Code,
        _ => SourceType::PlainText,
    }
}
