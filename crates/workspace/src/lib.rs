use std::{
    collections::BTreeSet,
    fs,
    path::{Component, Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileDelta {
    pub path: String,
    pub action: String,
    pub content: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileDeltaSet {
    pub summary: String,
    pub rationale: Vec<String>,
    pub changes: Vec<FileDelta>,
}

#[derive(Debug, Error)]
pub enum WorkspaceError {
    #[error("file delta for `{path}` is missing content")]
    MissingDeltaContent { path: String },

    #[error("unsupported file delta action `{action}` for `{path}`")]
    UnsupportedDeltaAction { action: String, path: String },

    #[error("failed to create directory: {0}")]
    CreateDir(#[from] std::io::Error),

    #[error("failed to write `{path}`: {source}")]
    WriteFile {
        path: String,
        source: std::io::Error,
    },

    #[error("failed to delete `{path}`: {source}")]
    DeleteFile {
        path: String,
        source: std::io::Error,
    },

    #[error("file delta path `{0}` must stay within the project root")]
    UnsafePath(String),

    #[error("failed to read `{path}`: {source}")]
    ReadFile {
        path: String,
        source: std::io::Error,
    },

    #[error("failed to read directory `{path}`: {source}")]
    ReadDir {
        path: String,
        source: std::io::Error,
    },

    #[error("failed to iterate directory `{path}`: {source}")]
    IterateDir {
        path: String,
        source: std::io::Error,
    },

    #[error("failed to inspect `{path}`: {source}")]
    InspectFile {
        path: String,
        source: std::io::Error,
    },

    #[error("failed to strip prefix: {0}")]
    StripPrefix(#[from] std::path::StripPrefixError),

    #[error("failed to remove `{path}`: {source}")]
    RemoveDir {
        path: String,
        source: std::io::Error,
    },

    #[error("merged content for `{path}` was not valid UTF-8: {source}")]
    InvalidMergeUtf8 {
        path: String,
        source: std::string::FromUtf8Error,
    },

    #[error("failed to run merge for `{path}`: {source}")]
    MergeCommand {
        path: String,
        source: std::io::Error,
    },

    #[error("parallel changes to `{0}` produced merge conflicts")]
    MergeConflict(String),
}

pub fn apply_file_deltas(root: &Path, delta: &FileDeltaSet) -> Result<(), WorkspaceError> {
    for change in &delta.changes {
        let path = resolve_project_path(root, &change.path)?;
        match change.action.as_str() {
            "write" => {
                let Some(content) = &change.content else {
                    return Err(WorkspaceError::MissingDeltaContent {
                        path: change.path.clone(),
                    });
                };
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent).map_err(WorkspaceError::CreateDir)?;
                }
                fs::write(&path, content).map_err(|error| WorkspaceError::WriteFile {
                    path: change.path.clone(),
                    source: error,
                })?;
            }
            "delete" => {
                if path.exists() {
                    fs::remove_file(&path).map_err(|error| WorkspaceError::DeleteFile {
                        path: change.path.clone(),
                        source: error,
                    })?;
                }
            }
            other => {
                return Err(WorkspaceError::UnsupportedDeltaAction {
                    action: other.to_string(),
                    path: change.path.clone(),
                });
            }
        }
    }

    Ok(())
}

pub fn apply_single_change(project_root: &Path, change: &FileDelta) -> Result<(), WorkspaceError> {
    apply_file_deltas(
        project_root,
        &FileDeltaSet {
            summary: "apply merged worktree change".to_string(),
            rationale: Vec::new(),
            changes: vec![change.clone()],
        },
    )
}

pub fn build_workspace_delta(
    source_root: &Path,
    target_root: &Path,
) -> Result<Vec<FileDelta>, WorkspaceError> {
    let source_paths = collect_workspace_files(source_root)?;
    let target_paths = collect_workspace_files(target_root)?;
    let paths = source_paths
        .union(&target_paths)
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut changes = Vec::new();

    for path in paths {
        let source = read_optional_file(source_root, &path)?;
        let target = read_optional_file(target_root, &path)?;
        if source == target {
            continue;
        }

        changes.push(FileDelta {
            path: path.clone(),
            action: if source.is_some() {
                "write".to_string()
            } else {
                "delete".to_string()
            },
            content: source,
        });
    }

    Ok(changes)
}

pub fn collect_workspace_files(root: &Path) -> Result<BTreeSet<String>, WorkspaceError> {
    let mut paths = BTreeSet::new();
    collect_workspace_files_recursive(root, root, &mut paths)?;
    Ok(paths)
}

pub fn command_failure_summary(stdout: &[u8], stderr: &[u8]) -> String {
    let stderr = String::from_utf8_lossy(stderr).trim().to_string();
    if !stderr.is_empty() {
        return truncate_text(&stderr);
    }

    let stdout = String::from_utf8_lossy(stdout).trim().to_string();
    if !stdout.is_empty() {
        return truncate_text(&stdout);
    }

    "command exited unsuccessfully with no output".to_string()
}

pub fn create_baseline_snapshot(
    project_root: &Path,
    name: &str,
) -> Result<PathBuf, WorkspaceError> {
    let snapshot_root = worktree_path(project_root, name);
    remove_directory_if_exists(&snapshot_root)?;
    fs::create_dir_all(&snapshot_root).map_err(WorkspaceError::CreateDir)?;
    sync_workspace_state(project_root, &snapshot_root)?;
    Ok(snapshot_root)
}

pub async fn merge_change_into_workspace(
    project_root: &Path,
    baseline_root: &Path,
    item_root: &Path,
    change: &FileDelta,
) -> Result<(), WorkspaceError> {
    let current = read_optional_file(project_root, &change.path)?;
    let base = read_optional_file(baseline_root, &change.path)?;

    let item_content = read_optional_file(item_root, &change.path)?;

    match (base, current.clone(), item_content) {
        (Some(base_content), Some(current_content), Some(item_content)) => {
            let merged = merge_file_versions(
                project_root,
                &change.path,
                &current_content,
                &base_content,
                &item_content,
            )
            .await?;
            write_workspace_file(project_root, &change.path, &merged)?;
            Ok(())
        }
        (None, None, Some(item_content)) => {
            write_workspace_file(project_root, &change.path, &item_content)?;
            Ok(())
        }
        (Some(_base), Some(current_content), None) => {
            write_workspace_file(project_root, &change.path, &current_content)?;
            Ok(())
        }
        (None, Some(current), Some(item)) if current == item => Ok(()),
        (Some(_base), None, None) => Ok(()),
        _ => Err(WorkspaceError::MergeConflict(change.path.clone())),
    }
}

pub async fn merge_file_versions(
    project_root: &Path,
    path: &str,
    current: &str,
    base: &str,
    item: &str,
) -> Result<String, WorkspaceError> {
    let temp_root = worktree_path(project_root, "merge-temp");
    fs::create_dir_all(&temp_root).map_err(WorkspaceError::CreateDir)?;
    let current_path = temp_root.join("current.tmp");
    let base_path = temp_root.join("base.tmp");
    let item_path = temp_root.join("item.tmp");
    fs::write(&current_path, current).map_err(|error| WorkspaceError::WriteFile {
        path: "current.tmp".to_string(),
        source: error,
    })?;
    fs::write(&base_path, base).map_err(|error| WorkspaceError::WriteFile {
        path: "base.tmp".to_string(),
        source: error,
    })?;
    fs::write(&item_path, item).map_err(|error| WorkspaceError::WriteFile {
        path: "item.tmp".to_string(),
        source: error,
    })?;

    let output = tokio::process::Command::new("git")
        .args([
            "merge-file",
            "-p",
            current_path.to_string_lossy().as_ref(),
            base_path.to_string_lossy().as_ref(),
            item_path.to_string_lossy().as_ref(),
        ])
        .current_dir(project_root)
        .output()
        .await
        .map_err(|error| WorkspaceError::MergeCommand {
            path: path.to_string(),
            source: error,
        })?;
    remove_directory_if_exists(&temp_root)?;

    if !output.status.success() {
        return Err(WorkspaceError::MergeConflict(path.to_string()));
    }

    String::from_utf8(output.stdout).map_err(|error| WorkspaceError::InvalidMergeUtf8 {
        path: path.to_string(),
        source: error,
    })
}

pub async fn merge_item_worktree(
    project_root: &Path,
    baseline_root: &Path,
    worktree_name: &str,
    _delta: &FileDeltaSet,
) -> Result<Vec<FileDelta>, WorkspaceError> {
    let item_root = worktree_path(project_root, worktree_name);
    let changes = build_workspace_delta(&item_root, baseline_root)?;

    for change in &changes {
        merge_change_into_workspace(project_root, baseline_root, &item_root, change).await?;
    }

    remove_worktree(project_root, worktree_name).await?;

    Ok(changes)
}

pub async fn prepare_worktree(
    project_root: &Path,
    worktree_name: &str,
) -> Result<PathBuf, WorkspaceError> {
    let worktree_root = worktree_path(project_root, worktree_name);
    if worktree_root.exists() {
        remove_worktree(project_root, worktree_name).await?;
    }

    let worktree_parent = worktree_root
        .parent()
        .expect("worktree path should have a parent");
    fs::create_dir_all(worktree_parent).map_err(WorkspaceError::CreateDir)?;

    run_git_command(
        project_root,
        &[
            "worktree",
            "add",
            "--detach",
            worktree_root.to_string_lossy().as_ref(),
            "HEAD",
        ],
        "create isolated worktree",
    )
    .await?;

    sync_workspace_state(project_root, &worktree_root)?;
    Ok(worktree_root)
}

pub fn read_optional_file(root: &Path, relative: &str) -> Result<Option<String>, WorkspaceError> {
    let path = root.join(relative);
    if !path.exists() {
        return Ok(None);
    }

    fs::read_to_string(&path)
        .map(Some)
        .map_err(|error| WorkspaceError::ReadFile {
            path: path.display().to_string(),
            source: error,
        })
}

pub fn remove_directory_if_exists(path: &Path) -> Result<(), WorkspaceError> {
    if path.exists() {
        fs::remove_dir_all(path).map_err(|error| WorkspaceError::RemoveDir {
            path: path.display().to_string(),
            source: error,
        })?;
    }
    Ok(())
}

pub async fn remove_worktree(
    project_root: &Path,
    worktree_name: &str,
) -> Result<(), WorkspaceError> {
    let worktree_root = worktree_path(project_root, worktree_name);
    if !worktree_root.exists() {
        return Ok(());
    }

    let output = tokio::process::Command::new("git")
        .args([
            "worktree",
            "remove",
            "--force",
            worktree_root.to_string_lossy().as_ref(),
        ])
        .current_dir(project_root)
        .output()
        .await
        .map_err(|error| WorkspaceError::MergeCommand {
            path: worktree_root.display().to_string(),
            source: error,
        })?;

    if !output.status.success() && worktree_root.exists() {
        fs::remove_dir_all(&worktree_root).map_err(|error| WorkspaceError::RemoveDir {
            path: worktree_root.display().to_string(),
            source: error,
        })?;
    }

    Ok(())
}

pub fn resolve_project_path(root: &Path, relative: &str) -> Result<PathBuf, WorkspaceError> {
    let path = Path::new(relative);
    if path.components().any(|component| {
        matches!(
            component,
            Component::Prefix(_) | Component::RootDir | Component::ParentDir
        )
    }) {
        return Err(WorkspaceError::UnsafePath(relative.to_string()));
    }

    let resolved = root.join(path);
    if !resolved.starts_with(root) {
        return Err(WorkspaceError::UnsafePath(relative.to_string()));
    }

    Ok(resolved)
}

pub async fn run_git_command(
    project_root: &Path,
    args: &[&str],
    label: &str,
) -> Result<(), WorkspaceError> {
    let output = tokio::process::Command::new("git")
        .args(args)
        .current_dir(project_root)
        .output()
        .await
        .map_err(|error| WorkspaceError::MergeCommand {
            path: label.to_string(),
            source: error,
        })?;

    if output.status.success() {
        return Ok(());
    }

    Err(WorkspaceError::MergeCommand {
        path: label.to_string(),
        source: std::io::Error::other(command_failure_summary(&output.stdout, &output.stderr)),
    })
}

pub fn should_skip_workspace_entry(name: &str) -> bool {
    matches!(name, ".git" | "target")
}

pub fn sync_workspace_state(source_root: &Path, target_root: &Path) -> Result<(), WorkspaceError> {
    let delta = build_workspace_delta(source_root, target_root)?;
    apply_file_deltas(
        target_root,
        &FileDeltaSet {
            summary: "sync workspace state".to_string(),
            rationale: Vec::new(),
            changes: delta,
        },
    )
}

pub fn truncate_text(text: &str) -> String {
    const MAX_LEN: usize = 600;
    if text.len() <= MAX_LEN {
        text.to_string()
    } else {
        format!("{}...", &text[..MAX_LEN])
    }
}

pub fn worktree_path(project_root: &Path, worktree_name: &str) -> PathBuf {
    project_root.join(".naaf-worktrees").join(worktree_name)
}

pub fn write_workspace_file(
    project_root: &Path,
    relative: &str,
    content: &str,
) -> Result<(), WorkspaceError> {
    let path = resolve_project_path(project_root, relative)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(WorkspaceError::CreateDir)?;
    }
    fs::write(path, content).map_err(|error| WorkspaceError::WriteFile {
        path: relative.to_string(),
        source: error,
    })
}

fn collect_workspace_files_recursive(
    root: &Path,
    current: &Path,
    paths: &mut BTreeSet<String>,
) -> Result<(), WorkspaceError> {
    for entry in fs::read_dir(current).map_err(|error| WorkspaceError::ReadDir {
        path: current.display().to_string(),
        source: error,
    })? {
        let entry = entry.map_err(|error| WorkspaceError::IterateDir {
            path: current.display().to_string(),
            source: error,
        })?;
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if should_skip_workspace_entry(&file_name) {
            continue;
        }

        let file_type = entry
            .file_type()
            .map_err(|error| WorkspaceError::InspectFile {
                path: path.display().to_string(),
                source: error,
            })?;
        if file_type.is_dir() {
            collect_workspace_files_recursive(root, &path, paths)?;
        } else if file_type.is_file() {
            let relative = path.strip_prefix(root)?;
            paths.insert(relative.to_string_lossy().replace('\\', "/"));
        }
    }

    Ok(())
}
