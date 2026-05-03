//! Filesystem-based persistence for `naaf`.
//!
//! `naaf-persistence-fs` provides generic artifact storage and a knowledge-group
//! store backed by the local filesystem.
//!
//! # Artifact Store
//!
//! For generic typed artifact persistence, use `ArtifactStore`:
//!
//! ```ignore
//! use naaf_persistence_fs::ArtifactStore;
//!
//! let store = ArtifactStore::create("/tmp/my-run")?;
//! store.write_json("plan.json", &my_plan)?;
//! ```

use std::{
    fs,
    path::{Path, PathBuf},
    process,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use naaf_knowledge::{KnowledgeGroup, KnowledgeGroupStore, KnowledgeGroupStoreFuture};
use serde::Serialize;
/// Generic filesystem artifact store for typed JSON artifacts.
///
/// Creates a directory at `run_root` and writes serialisable values
/// to named JSON files within it. Sub-directories are created on demand
/// when the artifact name contains path separators.
#[derive(Clone, Debug)]
pub struct ArtifactStore {
    run_root: PathBuf,
}

/// Filesystem-backed persistence for knowledge-group metadata.
#[derive(Clone, Debug)]
pub struct FsKnowledgeGroupStore {
    base_dir: Arc<PathBuf>,
}

impl ArtifactStore {
    /// Creates a new artifact store at `run_root`, creating the directory
    /// and any missing parents.
    pub fn create(run_root: impl Into<PathBuf>) -> std::io::Result<Self> {
        let run_root = run_root.into();
        fs::create_dir_all(&run_root)?;
        Ok(Self { run_root })
    }

    /// Returns the root directory for this store.
    pub fn run_root(&self) -> &Path {
        &self.run_root
    }

    /// Serialises `value` as pretty JSON and writes it to `artifact_name`
    /// relative to the run root.
    pub fn write_json<T: Serialize + ?Sized>(
        &self,
        artifact_name: &str,
        value: &T,
    ) -> std::io::Result<()> {
        let path = self.run_root.join(artifact_name);
        let payload = serde_json::to_string_pretty(value).map_err(|error| {
            std::io::Error::other(format!(
                "failed to serialise artifact `{artifact_name}`: {error}"
            ))
        })?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, payload)
    }

    /// Reads and deserialises a previously written artifact.
    pub fn read_json<T: serde::de::DeserializeOwned>(
        &self,
        artifact_name: &str,
    ) -> std::io::Result<Option<T>> {
        let path = self.run_root.join(artifact_name);
        match fs::read_to_string(&path) {
            Ok(data) => serde_json::from_str(&data).map(Some).map_err(|error| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("failed to parse artifact `{artifact_name}`: {error}"),
                )
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }
}

impl FsKnowledgeGroupStore {
    /// Creates a new knowledge-group store rooted at `base_dir`.
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: Arc::new(base_dir.into()),
        }
    }

    fn groups_dir(&self) -> PathBuf {
        self.base_dir.join("knowledge-groups")
    }

    fn group_path(&self, collection: &str) -> PathBuf {
        self.groups_dir()
            .join(format!("{}.json", encode_collection_filename(collection)))
    }
}

impl KnowledgeGroupStore for FsKnowledgeGroupStore {
    fn upsert_group(&self, group: &KnowledgeGroup) -> KnowledgeGroupStoreFuture<()> {
        let path = self.group_path(&group.collection);
        let group = group.clone();
        Box::pin(async move {
            let existing = match tokio::fs::read_to_string(&path).await {
                Ok(data) => Some(serde_json::from_str::<KnowledgeGroup>(&data)?),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => {
                    return Err(Box::new(error) as Box<dyn std::error::Error + Send + Sync>);
                }
            };
            let group = group.prepare_for_upsert(existing.as_ref());
            let data = serde_json::to_string_pretty(&group)?;
            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::write(&path, data).await?;
            Ok(())
        })
    }

    fn load_group(&self, collection: &str) -> KnowledgeGroupStoreFuture<Option<KnowledgeGroup>> {
        let path = self.group_path(collection);
        Box::pin(async move {
            match tokio::fs::read_to_string(&path).await {
                Ok(data) => Ok(Some(serde_json::from_str(&data)?)),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(error) => Err(Box::new(error) as Box<dyn std::error::Error + Send + Sync>),
            }
        })
    }

    fn list_groups(&self) -> KnowledgeGroupStoreFuture<Vec<KnowledgeGroup>> {
        let groups_dir = self.groups_dir();
        Box::pin(async move {
            let mut groups = Vec::new();
            let mut entries = match tokio::fs::read_dir(&groups_dir).await {
                Ok(entries) => entries,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(groups),
                Err(error) => {
                    return Err(Box::new(error) as Box<dyn std::error::Error + Send + Sync>);
                }
            };

            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                let is_json = path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extension == "json");
                if !is_json {
                    continue;
                }

                let data = tokio::fs::read_to_string(&path).await?;
                groups.push(serde_json::from_str::<KnowledgeGroup>(&data)?);
            }

            groups.sort_by(|left, right| left.collection.cmp(&right.collection));
            Ok(groups)
        })
    }

    fn delete_group(&self, collection: &str) -> KnowledgeGroupStoreFuture<()> {
        let path = self.group_path(collection);
        Box::pin(async move {
            match tokio::fs::remove_file(&path).await {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(Box::new(error) as Box<dyn std::error::Error + Send + Sync>),
            }
        })
    }
}

/// Generates a unique run identifier suitable for use as a directory name.
pub fn generate_run_id() -> std::io::Result<String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| {
            std::io::Error::other(format!("failed to read system clock for run id: {error}"))
        })?;

    Ok(format!(
        "run-{}-{:09}-{}",
        now.as_secs(),
        now.subsec_nanos(),
        process::id()
    ))
}

fn encode_collection_filename(collection: &str) -> String {
    let mut encoded = String::with_capacity(collection.len() * 2);
    for byte in collection.bytes() {
        encoded.push_str(&format!("{byte:02x}"));
    }
    encoded
}
