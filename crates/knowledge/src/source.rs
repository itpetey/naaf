use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SourceType {
    Markdown,
    Code,
    Conversation,
    Paper,
    PlainText,
    Directory,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceInfo {
    pub source_type: SourceType,
    pub path: Option<PathBuf>,
    pub title: Option<String>,
    pub language: Option<String>,
    pub content: Option<String>,
}

impl SourceInfo {
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

    pub fn markdown(content: &str, title: Option<String>) -> Self {
        Self {
            source_type: SourceType::Markdown,
            path: None,
            title,
            language: None,
            content: Some(content.to_string()),
        }
    }

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

fn detect_source_type(path: &std::path::Path) -> SourceType {
    match path.extension().and_then(|e| e.to_str()) {
        Some("pdf") => SourceType::Paper,
        Some("md" | "txt") => SourceType::Markdown,
        Some("json") => SourceType::Conversation,
        Some("rs" | "py" | "ts" | "js" | "go" | "java" | "c" | "cpp" | "h") => SourceType::Code,
        _ => SourceType::PlainText,
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
