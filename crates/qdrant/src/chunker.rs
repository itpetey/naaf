use std::path::Path;

use crate::error::QdrantError;
use crate::payload::SourceType;

#[derive(Clone, Debug)]
pub struct Chunk {
    pub text: String,
    pub metadata: ChunkMetadata,
}

#[derive(Clone, Debug)]
pub struct ChunkMetadata {
    pub index: usize,
    pub source_type: SourceType,
    pub start_char: usize,
    pub end_char: usize,
    pub path: Option<String>,
    pub language: Option<String>,
    pub heading: Option<String>,
}

pub trait Chunker {
    fn chunk(&self, content: &str, source_info: &SourceInfo) -> Result<Vec<Chunk>, QdrantError>;
}

#[derive(Clone, Debug)]
pub struct SourceInfo {
    pub source_type: SourceType,
    pub path: Option<String>,
    pub language: Option<String>,
    pub title: Option<String>,
}

impl SourceInfo {
    pub fn from_path(path: &Path) -> Result<Self, QdrantError> {
        let source_type = detect_source_type(path);
        let language = detect_language(path);
        let title = path.file_stem().and_then(|s| s.to_str()).map(String::from);
        Ok(Self {
            source_type,
            path: Some(path.to_string_lossy().into_owned()),
            language,
            title,
        })
    }

    pub fn markdown(_content: &str, title: Option<String>) -> Self {
        Self {
            source_type: SourceType::Markdown,
            path: None,
            language: None,
            title: title.or_else(|| Some("untitled".to_string())),
        }
    }

    pub fn conversation(title: Option<String>) -> Self {
        Self {
            source_type: SourceType::Conversation,
            path: None,
            language: None,
            title,
        }
    }
}

fn detect_source_type(path: &Path) -> SourceType {
    match path.extension().and_then(|e| e.to_str()) {
        Some("pdf") => SourceType::Paper,
        Some("md" | "txt") => SourceType::Markdown,
        Some("json") => SourceType::Conversation,
        Some("rs" | "py" | "ts" | "js" | "go" | "java" | "c" | "cpp" | "h") => SourceType::Code,
        _ => SourceType::PlainText,
    }
}

fn detect_language(path: &Path) -> Option<String> {
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

pub struct MarkdownChunker {
    max_chunk_size: usize,
    overlap: usize,
}

impl MarkdownChunker {
    pub fn new(max_chunk_size: usize, overlap: usize) -> Self {
        Self {
            max_chunk_size,
            overlap,
        }
    }
}

impl Default for MarkdownChunker {
    fn default() -> Self {
        Self::new(1000, 200)
    }
}

impl Chunker for MarkdownChunker {
    fn chunk(&self, content: &str, source_info: &SourceInfo) -> Result<Vec<Chunk>, QdrantError> {
        let sections = split_by_headings(content);
        let mut chunks = Vec::new();
        let mut buffer = String::new();
        let mut buffer_start = 0;
        let mut current_heading: Option<String> = None;
        let mut char_offset = 0;

        for section in &sections {
            current_heading = section.heading.clone().or(current_heading.clone());

            if buffer.len() + section.content.len() > self.max_chunk_size && !buffer.is_empty() {
                chunks.push(Chunk {
                    text: buffer.trim().to_string(),
                    metadata: ChunkMetadata {
                        index: chunks.len(),
                        source_type: source_info.source_type.clone(),
                        start_char: buffer_start,
                        end_char: char_offset,
                        path: source_info.path.clone(),
                        language: source_info.language.clone(),
                        heading: current_heading.clone(),
                    },
                });
                if self.overlap > 0 && buffer.len() > self.overlap {
                    buffer = buffer[buffer.len().saturating_sub(self.overlap)..].to_string();
                } else {
                    buffer.clear();
                }
                buffer_start = char_offset.saturating_sub(self.overlap);
            }

            buffer.push_str(&section.content);
            buffer.push('\n');
            char_offset += section.content.len() + 1;
        }

        if !buffer.trim().is_empty() {
            chunks.push(Chunk {
                text: buffer.trim().to_string(),
                metadata: ChunkMetadata {
                    index: chunks.len(),
                    source_type: source_info.source_type.clone(),
                    start_char: buffer_start,
                    end_char: char_offset,
                    path: source_info.path.clone(),
                    language: source_info.language.clone(),
                    heading: current_heading,
                },
            });
        }

        Ok(chunks)
    }
}

struct Section {
    heading: Option<String>,
    content: String,
}

fn split_by_headings(content: &str) -> Vec<Section> {
    let mut sections = Vec::new();
    let mut current_heading: Option<String> = None;
    let mut current_content = String::new();

    for line in content.lines() {
        if line.starts_with('#') {
            if !current_content.trim().is_empty() || current_heading.is_some() {
                sections.push(Section {
                    heading: current_heading.take(),
                    content: std::mem::take(&mut current_content),
                });
            }
            current_heading = Some(line.trim_start_matches('#').trim().to_string());
        } else {
            current_content.push_str(line);
            current_content.push('\n');
        }
    }

    if !current_content.trim().is_empty() || current_heading.is_some() {
        sections.push(Section {
            heading: current_heading,
            content: current_content,
        });
    }

    sections
}

pub struct CodeChunker {
    max_chunk_size: usize,
}

impl CodeChunker {
    pub fn new(max_chunk_size: usize) -> Self {
        Self { max_chunk_size }
    }
}

impl Default for CodeChunker {
    fn default() -> Self {
        Self::new(1500)
    }
}

impl Chunker for CodeChunker {
    fn chunk(&self, content: &str, source_info: &SourceInfo) -> Result<Vec<Chunk>, QdrantError> {
        let mut chunks = Vec::new();
        let mut current = String::new();
        let mut start = 0;

        for line in content.lines() {
            let trimmed = line.trim();

            let is_boundary = trimmed.starts_with("pub ")
                || trimmed.starts_with("fn ")
                || trimmed.starts_with("async fn ")
                || trimmed.starts_with("impl ")
                || trimmed.starts_with("struct ")
                || trimmed.starts_with("enum ")
                || trimmed.starts_with("trait ")
                || trimmed.starts_with("class ")
                || trimmed.starts_with("def ")
                || trimmed.starts_with('#');

            if is_boundary
                && !current.is_empty()
                && (current.len() > self.max_chunk_size
                    || current.len() + line.len() > self.max_chunk_size)
            {
                chunks.push(Chunk {
                    text: current.trim().to_string(),
                    metadata: ChunkMetadata {
                        index: chunks.len(),
                        source_type: source_info.source_type.clone(),
                        start_char: start,
                        end_char: start + current.len(),
                        path: source_info.path.clone(),
                        language: source_info.language.clone(),
                        heading: None,
                    },
                });
                start += current.len();
                current.clear();
            }

            current.push_str(line);
            current.push('\n');
        }

        if !current.trim().is_empty() {
            chunks.push(Chunk {
                text: current.trim().to_string(),
                metadata: ChunkMetadata {
                    index: chunks.len(),
                    source_type: source_info.source_type.clone(),
                    start_char: start,
                    end_char: start + current.len(),
                    path: source_info.path.clone(),
                    language: source_info.language.clone(),
                    heading: None,
                },
            });
        }

        Ok(chunks)
    }
}

pub struct ConversationChunker {
    max_chunk_size: usize,
}

impl ConversationChunker {
    pub fn new(max_chunk_size: usize) -> Self {
        Self { max_chunk_size }
    }
}

impl Default for ConversationChunker {
    fn default() -> Self {
        Self::new(2000)
    }
}

impl Chunker for ConversationChunker {
    fn chunk(&self, content: &str, source_info: &SourceInfo) -> Result<Vec<Chunk>, QdrantError> {
        let messages: Vec<crate::conversation::Message> = serde_json::from_str(content)
            .map_err(|e| QdrantError::Chunking(format!("failed to parse conversation: {e}")))?;

        let mut chunks = Vec::new();
        let mut buffer = String::new();
        let mut char_offset = 0;

        for message in &messages {
            let entry = format!("{}: {}\n", message.role, message.content);
            if buffer.len() + entry.len() > self.max_chunk_size && !buffer.is_empty() {
                chunks.push(Chunk {
                    text: buffer.trim().to_string(),
                    metadata: ChunkMetadata {
                        index: chunks.len(),
                        source_type: source_info.source_type.clone(),
                        start_char: char_offset,
                        end_char: char_offset + buffer.len(),
                        path: source_info.path.clone(),
                        language: None,
                        heading: None,
                    },
                });
                char_offset += buffer.len();
                buffer.clear();
            }
            buffer.push_str(&entry);
        }

        if !buffer.trim().is_empty() {
            chunks.push(Chunk {
                text: buffer.trim().to_string(),
                metadata: ChunkMetadata {
                    index: chunks.len(),
                    source_type: source_info.source_type.clone(),
                    start_char: char_offset,
                    end_char: char_offset + buffer.len(),
                    path: source_info.path.clone(),
                    language: None,
                    heading: None,
                },
            });
        }

        Ok(chunks)
    }
}

pub struct PdfChunker {
    inner: MarkdownChunker,
}

impl PdfChunker {
    pub fn new(max_chunk_size: usize, overlap: usize) -> Self {
        Self {
            inner: MarkdownChunker::new(max_chunk_size, overlap),
        }
    }

    pub fn extract_text(&self, path: &Path) -> Result<String, QdrantError> {
        pdf_extract::extract_text(path)
            .map_err(|e| QdrantError::PdfExtraction(format!("failed to extract PDF text: {e}")))
    }
}

impl Default for PdfChunker {
    fn default() -> Self {
        Self::new(1000, 200)
    }
}

impl Chunker for PdfChunker {
    fn chunk(&self, content: &str, source_info: &SourceInfo) -> Result<Vec<Chunk>, QdrantError> {
        let pdf_source_info = SourceInfo {
            source_type: SourceType::Paper,
            ..source_info.clone()
        };
        self.inner.chunk(content, &pdf_source_info)
    }
}

pub enum ContentChunker {
    Markdown(MarkdownChunker),
    Code(CodeChunker),
    Conversation(ConversationChunker),
    Pdf(PdfChunker),
}

impl ContentChunker {
    pub fn from_path(path: &Path) -> Result<Self, QdrantError> {
        match path.extension().and_then(|e| e.to_str()) {
            Some("pdf") => Ok(Self::Pdf(PdfChunker::default())),
            Some("md" | "txt") => Ok(Self::Markdown(MarkdownChunker::default())),
            Some("json") => Ok(Self::Conversation(ConversationChunker::default())),
            Some("rs" | "py" | "ts" | "js" | "go" | "java" | "c" | "cpp" | "h") => {
                Ok(Self::Code(CodeChunker::default()))
            }
            _ => Ok(Self::Markdown(MarkdownChunker::default())),
        }
    }
}

impl Chunker for ContentChunker {
    fn chunk(&self, content: &str, source_info: &SourceInfo) -> Result<Vec<Chunk>, QdrantError> {
        match self {
            Self::Markdown(c) => c.chunk(content, source_info),
            Self::Code(c) => c.chunk(content, source_info),
            Self::Conversation(c) => c.chunk(content, source_info),
            Self::Pdf(c) => c.chunk(content, source_info),
        }
    }
}
