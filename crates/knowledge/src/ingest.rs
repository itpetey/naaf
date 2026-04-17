use std::path::Path;

use ignore::WalkBuilder;
use naaf_qdrant::{
    Chunker, ContentChunker, QdrantAgent, SourceInfo as QdrantSourceInfo,
    SourceType as QdrantSourceType,
};

use crate::error::KnowledgeError;
use crate::knowledge::IngestReport;
use crate::source::{SourceInfo, SourceType};

static BINARY_EXTENSIONS: &[&str] = &[
    ".lock", ".min.js", ".min.css", ".map", ".pyc", ".pyo", ".so", ".dylib", ".dll", ".exe", ".o",
    ".a", ".wasm", ".png", ".jpg", ".jpeg", ".gif", ".ico", ".svg", ".woff", ".woff2", ".ttf",
    ".eot", ".gz", ".zip", ".tar", ".rar", ".7z", ".bz2", ".xz", ".zst", ".class", ".jar", ".war",
];

pub async fn ingest_file<R>(
    qdrant: &QdrantAgent<R>,
    path: &Path,
    repo: Option<&str>,
) -> Result<IngestReport, KnowledgeError> {
    let source_info = SourceInfo::from_path(path)?;
    let content = std::fs::read_to_string(path)?;

    let qdrant_source_info = convert_source_info(&source_info)?;

    let chunker =
        ContentChunker::from_path(path).map_err(|e| KnowledgeError::Ingest(e.to_string()))?;

    let chunks = chunker
        .chunk(&content, &qdrant_source_info)
        .map_err(|e| KnowledgeError::Ingest(e.to_string()))?;

    let chunks_count = chunks.len();

    let source_ids = qdrant
        .upsert_chunks(chunks, &qdrant_source_info, repo)
        .await
        .map_err(KnowledgeError::Qdrant)?;

    Ok(IngestReport {
        source_ids,
        knowledge_ids: Vec::new(),
        chunks_count,
        entries_count: 0,
    })
}

pub async fn ingest_directory<R>(
    qdrant: &QdrantAgent<R>,
    dir: &Path,
    repo: Option<&str>,
) -> Result<DirectoryIngestReport, KnowledgeError> {
    let mut reports = Vec::new();
    let mut total_chunks = 0;
    let mut total_source_ids = Vec::new();
    let mut files_ingested = 0;
    let mut files_skipped = 0;

    for entry in walk_dir(dir) {
        match ingest_file(qdrant, &entry, repo).await {
            Ok(report) => {
                total_chunks += report.chunks_count;
                total_source_ids.extend(report.source_ids.iter().copied());
                files_ingested += 1;
                reports.push((entry, Ok(report)));
            }
            Err(e) => {
                files_skipped += 1;
                reports.push((entry, Err(e)));
            }
        }
    }

    Ok(DirectoryIngestReport {
        files_ingested,
        files_skipped,
        total_chunks,
        total_source_ids,
        reports,
    })
}

pub async fn ingest_content<R>(
    qdrant: &QdrantAgent<R>,
    content: &str,
    source_info: &SourceInfo,
    repo: Option<&str>,
) -> Result<IngestReport, KnowledgeError> {
    let qdrant_source_info = convert_source_info(source_info)?;

    let chunker = ContentChunker::Markdown(naaf_qdrant::MarkdownChunker::default());
    let chunks = chunker
        .chunk(content, &qdrant_source_info)
        .map_err(|e| KnowledgeError::Ingest(e.to_string()))?;

    let chunks_count = chunks.len();

    let source_ids = qdrant
        .upsert_chunks(chunks, &qdrant_source_info, repo)
        .await
        .map_err(KnowledgeError::Qdrant)?;

    Ok(IngestReport {
        source_ids,
        knowledge_ids: Vec::new(),
        chunks_count,
        entries_count: 0,
    })
}

fn convert_source_info(source_info: &SourceInfo) -> Result<QdrantSourceInfo, KnowledgeError> {
    Ok(QdrantSourceInfo {
        source_type: match source_info.source_type {
            SourceType::Markdown => QdrantSourceType::Markdown,
            SourceType::Code => QdrantSourceType::Code,
            SourceType::Conversation => QdrantSourceType::Conversation,
            SourceType::Paper => QdrantSourceType::Paper,
            SourceType::PlainText => QdrantSourceType::PlainText,
            SourceType::Directory => QdrantSourceType::Code,
        },
        path: source_info
            .path
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned()),
        language: source_info.language.clone(),
        title: source_info.title.clone(),
    })
}

fn walk_dir(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();

    let walker = WalkBuilder::new(dir)
        .hidden(false)
        .git_ignore(true)
        .git_exclude(true)
        .git_global(true)
        .build();

    for result in walker {
        let Ok(entry) = result else {
            continue;
        };
        let path = entry.path().to_path_buf();

        if !path.is_file() {
            continue;
        }

        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };

        if ext == "lock" {
            continue;
        }

        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if BINARY_EXTENSIONS
            .iter()
            .any(|ignored| file_name.ends_with(ignored))
        {
            continue;
        }

        if ext.is_empty() {
            continue;
        }

        files.push(path);
    }

    files.sort();
    files
}

#[derive(Debug)]
pub struct DirectoryIngestReport {
    pub files_ingested: usize,
    pub files_skipped: usize,
    pub total_chunks: usize,
    pub total_source_ids: Vec<uuid::Uuid>,
    pub reports: Vec<(std::path::PathBuf, Result<IngestReport, KnowledgeError>)>,
}
