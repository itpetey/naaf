use std::{
    convert::Infallible,
    fs,
    marker::PhantomData,
    path::{Component, Path, PathBuf},
};

use futures::future::LocalBoxFuture;
use glob::Pattern;
use serde::Deserialize;
use serde_json::Value;

use crate::message::ToolSpec;
use crate::tool::Tool;

const DEFAULT_MAX_LINES: usize = 200;
const DEFAULT_MAX_RESULTS: usize = 50;

#[derive(Clone, Debug, Deserialize)]
struct ReadFileParams {
    path: String,
    #[serde(default)]
    start_line: usize,
    #[serde(default = "default_max_lines")]
    max_lines: usize,
}

#[derive(Clone, Debug, Deserialize)]
struct GlobPathsParams {
    pattern: String,
    #[serde(default = "default_max_results")]
    max_results: usize,
}

#[derive(Clone, Debug, Deserialize)]
struct SearchFilesParams {
    query: String,
    #[serde(default)]
    include: Option<String>,
    #[serde(default = "default_max_results")]
    max_results: usize,
    #[serde(default)]
    case_sensitive: bool,
}

/// Tool that reads a workspace file and returns numbered lines.
pub struct ReadFileTool<R> {
    root: PathBuf,
    _marker: PhantomData<R>,
}

/// Tool that lists workspace paths matching a glob pattern.
pub struct GlobPathsTool<R> {
    root: PathBuf,
    _marker: PhantomData<R>,
}

/// Tool that performs literal text search across workspace files.
pub struct SearchFilesTool<R> {
    root: PathBuf,
    _marker: PhantomData<R>,
}

impl<R> ReadFileTool<R> {
    /// Creates a file-reading tool rooted at the given workspace path.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            _marker: PhantomData,
        }
    }
}

impl<R> Tool for ReadFileTool<R> {
    type Runtime = R;
    type Error = Infallible;

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "read_file".to_string(),
            description: "Read a text file from the workspace. Returns numbered lines for the requested file."
                .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Repository-relative path to read",
                    },
                    "start_line": {
                        "type": "integer",
                        "description": "Zero-based line offset to begin reading from",
                    },
                    "max_lines": {
                        "type": "integer",
                        "description": "Maximum number of lines to return",
                    },
                },
                "required": ["path"],
            }),
        }
    }

    fn call<'a>(
        &'a self,
        _runtime: &'a Self::Runtime,
        arguments: Value,
    ) -> LocalBoxFuture<'a, Result<Value, Self::Error>> {
        Box::pin(async move {
            let result = match serde_json::from_value::<ReadFileParams>(arguments) {
                Ok(params) => read_file(&self.root, params),
                Err(error) => serde_json::json!({
                    "error": format!("invalid arguments: {error}"),
                }),
            };
            Ok(result)
        })
    }
}

impl<R> GlobPathsTool<R> {
    /// Creates a glob tool rooted at the given workspace path.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            _marker: PhantomData,
        }
    }
}

impl<R> Tool for GlobPathsTool<R> {
    type Runtime = R;
    type Error = Infallible;

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "glob_paths".to_string(),
            description: "List workspace paths that match a glob pattern such as src/**/*.rs."
                .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Glob pattern matched against repository-relative paths",
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum number of matching paths to return",
                    },
                },
                "required": ["pattern"],
            }),
        }
    }

    fn call<'a>(
        &'a self,
        _runtime: &'a Self::Runtime,
        arguments: Value,
    ) -> LocalBoxFuture<'a, Result<Value, Self::Error>> {
        Box::pin(async move {
            let result = match serde_json::from_value::<GlobPathsParams>(arguments) {
                Ok(params) => glob_paths(&self.root, params),
                Err(error) => serde_json::json!({
                    "error": format!("invalid arguments: {error}"),
                }),
            };
            Ok(result)
        })
    }
}

impl<R> SearchFilesTool<R> {
    /// Creates a search tool rooted at the given workspace path.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            _marker: PhantomData,
        }
    }
}

impl<R> Tool for SearchFilesTool<R> {
    type Runtime = R;
    type Error = Infallible;

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "search_files".to_string(),
            description:
                "Search workspace text files for a literal string and return matching lines."
                    .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Literal text to search for in workspace files",
                    },
                    "include": {
                        "type": "string",
                        "description": "Optional glob pattern to limit which paths are searched",
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum number of matching lines to return",
                    },
                    "case_sensitive": {
                        "type": "boolean",
                        "description": "Whether the literal search should be case-sensitive",
                    },
                },
                "required": ["query"],
            }),
        }
    }

    fn call<'a>(
        &'a self,
        _runtime: &'a Self::Runtime,
        arguments: Value,
    ) -> LocalBoxFuture<'a, Result<Value, Self::Error>> {
        Box::pin(async move {
            let result = match serde_json::from_value::<SearchFilesParams>(arguments) {
                Ok(params) => search_files(&self.root, params),
                Err(error) => serde_json::json!({
                    "error": format!("invalid arguments: {error}"),
                }),
            };
            Ok(result)
        })
    }
}

fn default_max_lines() -> usize {
    DEFAULT_MAX_LINES
}

fn default_max_results() -> usize {
    DEFAULT_MAX_RESULTS
}

fn error_value(message: impl Into<String>) -> Value {
    serde_json::json!({
        "error": message.into(),
    })
}

fn glob_paths(root: &Path, params: GlobPathsParams) -> Value {
    let pattern = match Pattern::new(&params.pattern) {
        Ok(pattern) => pattern,
        Err(error) => return error_value(format!("invalid glob pattern: {error}")),
    };

    let mut matches = Vec::new();
    let mut truncated = false;

    for path in walk_workspace(root) {
        let Ok(relative) = path.strip_prefix(root) else {
            continue;
        };
        let relative = relative_path_string(relative);
        if pattern.matches(&relative) {
            if matches.len() >= params.max_results {
                truncated = true;
                break;
            }
            matches.push(relative);
        }
    }

    serde_json::json!({
        "pattern": params.pattern,
        "matches": matches,
        "truncated": truncated,
    })
}

fn is_ignored_directory(name: &str) -> bool {
    matches!(name, ".git" | "target")
}

fn read_file(root: &Path, params: ReadFileParams) -> Value {
    let path = match resolve_workspace_path(root, &params.path) {
        Ok(path) => path,
        Err(message) => return error_value(message),
    };

    let content = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(error) => return error_value(format!("failed to read file: {error}")),
    };

    let all_lines: Vec<&str> = content.lines().collect();
    let start = params.start_line.min(all_lines.len());
    let end = start.saturating_add(params.max_lines).min(all_lines.len());
    let lines = all_lines[start..end]
        .iter()
        .enumerate()
        .map(|(offset, line)| format!("{}: {}", start + offset + 1, line))
        .collect::<Vec<_>>();

    serde_json::json!({
        "path": params.path,
        "start_line": start + 1,
        "end_line": end,
        "truncated": end < all_lines.len(),
        "content": lines.join("\n"),
    })
}

fn relative_path_string(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn resolve_workspace_path(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let relative_path = Path::new(relative);
    if relative_path.components().any(|component| {
        matches!(
            component,
            Component::Prefix(_) | Component::RootDir | Component::ParentDir
        )
    }) {
        return Err("path must stay within the workspace root".to_string());
    }

    let path = root.join(relative_path);
    if !path.starts_with(root) {
        return Err("path resolved outside the workspace root".to_string());
    }

    Ok(path)
}

fn search_files(root: &Path, params: SearchFilesParams) -> Value {
    let include_pattern = match params.include.as_ref() {
        Some(pattern) => match Pattern::new(pattern) {
            Ok(pattern) => Some(pattern),
            Err(error) => return error_value(format!("invalid include pattern: {error}")),
        },
        None => None,
    };

    let needle = if params.case_sensitive {
        params.query.clone()
    } else {
        params.query.to_lowercase()
    };

    let mut matches = Vec::new();
    let mut truncated = false;

    for path in walk_workspace(root) {
        if !path.is_file() {
            continue;
        }

        let Ok(relative_path) = path.strip_prefix(root) else {
            continue;
        };
        let relative = relative_path_string(relative_path);

        if include_pattern
            .as_ref()
            .is_some_and(|pattern| !pattern.matches(&relative))
        {
            continue;
        }

        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };

        for (index, line) in content.lines().enumerate() {
            let haystack = if params.case_sensitive {
                line.to_string()
            } else {
                line.to_lowercase()
            };

            if !haystack.contains(&needle) {
                continue;
            }

            if matches.len() >= params.max_results {
                truncated = true;
                break;
            }

            matches.push(serde_json::json!({
                "path": relative,
                "line_number": index + 1,
                "line": line,
            }));
        }

        if truncated {
            break;
        }
    }

    serde_json::json!({
        "query": params.query,
        "include": params.include,
        "matches": matches,
        "truncated": truncated,
    })
}

fn walk_workspace(root: &Path) -> Vec<PathBuf> {
    let mut stack = vec![root.to_path_buf()];
    let mut paths = Vec::new();

    while let Some(path) = stack.pop() {
        let Ok(entries) = fs::read_dir(&path) else {
            continue;
        };

        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(is_ignored_directory)
            {
                continue;
            }

            if entry_path.is_dir() {
                stack.push(entry_path.clone());
            }
            paths.push(entry_path);
        }
    }

    paths.sort();
    paths
}

#[cfg(test)]
mod tests {
    use std::{
        env, fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use serde_json::json;

    use super::{GlobPathsTool, ReadFileTool, SearchFilesTool};
    use crate::tool::Tool;

    #[derive(Debug)]
    struct StubRuntime;

    struct TestWorkspace {
        root: PathBuf,
    }

    impl TestWorkspace {
        fn new() -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock should work")
                .as_nanos();
            let root = env::temp_dir().join(format!("naaf-llm-repository-{unique}"));
            fs::create_dir_all(root.join("src/nested")).expect("test directories should build");
            fs::create_dir_all(root.join("target/debug")).expect("target directory should build");
            fs::write(root.join("src/lib.rs"), "fn alpha() {}\nfn beta() {}\n")
                .expect("lib file should write");
            fs::write(root.join("src/nested/mod.rs"), "pub fn beta() {}\n")
                .expect("nested file should write");
            fs::write(root.join("README.md"), "alpha beta\n").expect("readme should write");

            Self { root }
        }
    }

    impl Drop for TestWorkspace {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[tokio::test]
    async fn read_file_tool_reads_numbered_lines() {
        let workspace = TestWorkspace::new();
        let tool = ReadFileTool::<StubRuntime>::new(&workspace.root);
        let result = tool
            .call(
                &StubRuntime,
                json!({
                    "path": "src/lib.rs",
                    "start_line": 0,
                    "max_lines": 1,
                }),
            )
            .await
            .expect("tool should succeed");

        assert_eq!(result["path"], "src/lib.rs");
        assert_eq!(result["start_line"], 1);
        assert_eq!(result["content"], "1: fn alpha() {}");
        assert_eq!(result["truncated"], true);
    }

    #[tokio::test]
    async fn glob_paths_tool_matches_relative_paths() {
        let workspace = TestWorkspace::new();
        let tool = GlobPathsTool::<StubRuntime>::new(&workspace.root);
        let result = tool
            .call(
                &StubRuntime,
                json!({
                    "pattern": "src/**/*.rs",
                }),
            )
            .await
            .expect("tool should succeed");

        let matches = result["matches"]
            .as_array()
            .expect("matches should be an array");
        assert!(matches.iter().any(|value| value == "src/lib.rs"));
        assert!(matches.iter().any(|value| value == "src/nested/mod.rs"));
        assert!(!matches.iter().any(|value| value == "target/debug"));
    }

    #[tokio::test]
    async fn search_files_tool_returns_matching_lines() {
        let workspace = TestWorkspace::new();
        let tool = SearchFilesTool::<StubRuntime>::new(&workspace.root);
        let result = tool
            .call(
                &StubRuntime,
                json!({
                    "query": "beta",
                    "include": "src/**/*.rs",
                }),
            )
            .await
            .expect("tool should succeed");

        let matches = result["matches"]
            .as_array()
            .expect("matches should be an array");
        assert_eq!(matches.len(), 2);
        assert!(
            matches
                .iter()
                .all(|entry| entry["path"].as_str().unwrap().starts_with("src/"))
        );
    }
}
