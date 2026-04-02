//! Filesystem-backed artifact store.
//!
//! # Legacy Code
//!
//! This module is part of the legacy prototype runtime.
//! **Do not build new features on this code.**
//! See the repository root `LEGACY.md` for details.

use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::artifact::{Artifact, ArtifactId, ArtifactKind, ArtifactRef};
use crate::finding::{FindingId, FindingStatus};
use crate::run::RunId;

const ARTIFACTS_DIR: &str = "artifacts";
const FINDINGS_DIR: &str = "findings";

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("Artifact not found: {0}")]
    NotFound(ArtifactId),

    #[error("Finding not found: run={0}, finding={1}")]
    FindingNotFound(RunId, FindingId),

    #[error("Run not found: {0}")]
    RunNotFound(RunId),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type StoreResult<T> = Result<T, StoreError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactMetadata {
    pub id: ArtifactId,
    pub run_id: RunId,
    pub kind: ArtifactKind,
    pub parent_ids: Vec<ArtifactId>,
    pub content_filename: String,
    pub created_at: DateTime<Utc>,
}

pub struct ArtifactStore {
    root: PathBuf,
}

pub struct FindingStore {
    root: PathBuf,
}

impl ArtifactStore {
    pub fn new(root: impl Into<PathBuf>) -> StoreResult<Self> {
        Ok(Self { root: root.into() })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn artifacts_dir(&self, _run_id: RunId) -> PathBuf {
        self.root.join(ARTIFACTS_DIR)
    }

    pub fn save(&self, artifact: &Artifact, content: &[u8]) -> StoreResult<()> {
        let run_dir = self.artifacts_dir(artifact.run_id);
        fs::create_dir_all(&run_dir)?;

        let content_filename = format!("{}.bin", artifact.id.0);
        let content_path = run_dir.join(&content_filename);
        let mut file = BufWriter::new(File::create(&content_path)?);
        file.write_all(content)?;
        file.flush()?;

        let metadata = ArtifactMetadata {
            id: artifact.id,
            run_id: artifact.run_id,
            kind: artifact.kind,
            parent_ids: artifact.parent_ids.clone(),
            content_filename,
            created_at: artifact.created_at,
        };

        let metadata_path = run_dir.join(format!("{}.json", artifact.id.0));
        let file = BufWriter::new(File::create(&metadata_path)?);
        serde_json::to_writer(file, &metadata)?;

        Ok(())
    }

    pub fn load(&self, id: ArtifactId, run_id: RunId) -> StoreResult<(Artifact, Vec<u8>)> {
        let run_dir = self.artifacts_dir(run_id);

        let metadata_path = run_dir.join(format!("{}.json", id.0));
        let file = BufReader::new(File::open(&metadata_path)?);
        let metadata: ArtifactMetadata = serde_json::from_reader(file)?;

        if metadata.id != id {
            return Err(StoreError::NotFound(id));
        }

        let content_path = run_dir.join(&metadata.content_filename);
        let mut file = BufReader::new(File::open(&content_path)?);
        let mut content = Vec::new();
        file.read_to_end(&mut content)?;

        let artifact = Artifact {
            id: metadata.id,
            run_id: metadata.run_id,
            kind: metadata.kind,
            parent_ids: metadata.parent_ids,
            content_path,
            created_at: metadata.created_at,
        };

        Ok((artifact, content))
    }

    pub fn list(&self, run_id: RunId) -> StoreResult<Vec<ArtifactRef>> {
        let run_dir = self.artifacts_dir(run_id);
        if !run_dir.exists() {
            return Ok(Vec::new());
        }

        let mut artifacts = Vec::new();
        for entry in fs::read_dir(&run_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                let file = BufReader::new(File::open(&path)?);
                let metadata: ArtifactMetadata = serde_json::from_reader(file)?;
                artifacts.push(ArtifactRef::new(metadata.id, metadata.kind));
            }
        }
        Ok(artifacts)
    }

    pub fn list_metadata(&self, run_id: RunId) -> StoreResult<Vec<ArtifactMetadata>> {
        let run_dir = self.artifacts_dir(run_id);
        if !run_dir.exists() {
            return Ok(Vec::new());
        }

        let mut artifacts = Vec::new();
        for entry in fs::read_dir(&run_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                let file = BufReader::new(File::open(&path)?);
                let metadata: ArtifactMetadata = serde_json::from_reader(file)?;
                artifacts.push(metadata);
            }
        }
        artifacts.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        Ok(artifacts)
    }

    pub fn exists(&self, id: ArtifactId, run_id: RunId) -> bool {
        let run_dir = self.artifacts_dir(run_id);
        if !run_dir.exists() {
            return false;
        }
        let metadata_path = run_dir.join(format!("{}.json", id.0));
        metadata_path.exists()
    }

    pub fn delete_run(&self, run_id: RunId) -> StoreResult<()> {
        let run_dir = self.artifacts_dir(run_id);
        if run_dir.exists() {
            fs::remove_dir_all(run_dir)?;
        }
        Ok(())
    }
}

impl FindingStore {
    pub fn new(root: impl Into<PathBuf>) -> StoreResult<Self> {
        Ok(Self { root: root.into() })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn findings_dir(&self, _run_id: RunId) -> PathBuf {
        self.root.join(FINDINGS_DIR)
    }

    pub fn save(&self, finding: &crate::finding::Finding) -> StoreResult<()> {
        let run_dir = self.findings_dir(finding.run_id);
        fs::create_dir_all(&run_dir)?;

        let finding_path = run_dir.join(format!("{}.json", finding.id.0));
        let file = BufWriter::new(File::create(&finding_path)?);
        serde_json::to_writer(file, finding)?;

        Ok(())
    }

    pub fn load(&self, id: FindingId, run_id: RunId) -> StoreResult<crate::finding::Finding> {
        let run_dir = self.findings_dir(run_id);
        let finding_path = run_dir.join(format!("{}.json", id.0));

        let file = BufReader::new(File::open(&finding_path)?);
        let finding: crate::finding::Finding = serde_json::from_reader(file)?;

        if finding.id != id {
            return Err(StoreError::FindingNotFound(run_id, id));
        }

        Ok(finding)
    }

    pub fn list(&self, run_id: RunId) -> StoreResult<Vec<crate::finding::Finding>> {
        let run_dir = self.findings_dir(run_id);
        if !run_dir.exists() {
            return Ok(Vec::new());
        }

        let mut findings = Vec::new();
        for entry in fs::read_dir(&run_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                let file = BufReader::new(File::open(&path)?);
                let finding: crate::finding::Finding = serde_json::from_reader(file)?;
                findings.push(finding);
            }
        }
        Ok(findings)
    }

    pub fn list_by_status(
        &self,
        run_id: RunId,
        status: FindingStatus,
    ) -> StoreResult<Vec<crate::finding::Finding>> {
        let all = self.list(run_id)?;
        Ok(all.into_iter().filter(|f| f.status == status).collect())
    }

    pub fn update_status(
        &self,
        id: FindingId,
        run_id: RunId,
        status: FindingStatus,
    ) -> StoreResult<()> {
        let mut finding = self.load(id, run_id)?;
        finding.status = status;

        if status == FindingStatus::Resolved {
            finding.resolved_at = Some(chrono::Utc::now());
        }

        self.save(&finding)
    }

    pub fn delete(&self, id: FindingId, run_id: RunId) -> StoreResult<()> {
        let run_dir = self.findings_dir(run_id);
        let finding_path = run_dir.join(format!("{}.json", id.0));

        if !finding_path.exists() {
            return Err(StoreError::FindingNotFound(run_id, id));
        }

        fs::remove_file(finding_path)?;
        Ok(())
    }

    pub fn delete_run(&self, run_id: RunId) -> StoreResult<()> {
        let run_dir = self.findings_dir(run_id);
        if run_dir.exists() {
            fs::remove_dir_all(run_dir)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::artifact::ArtifactKind;

    #[test]
    fn test_save_and_load_artifact() {
        let temp = TempDir::new().unwrap();
        let store = ArtifactStore::new(temp.path()).unwrap();

        let run_id = RunId::new();
        let artifact = Artifact::new(
            run_id,
            ArtifactKind::UserPrompt,
            vec![],
            PathBuf::from("test.bin"),
        );
        let content = b"hello world";

        store.save(&artifact, content).unwrap();

        let loaded = store.load(artifact.id, run_id).unwrap();
        assert_eq!(loaded.1, content);
    }

    #[test]
    fn test_list_artifacts() {
        let temp = TempDir::new().unwrap();
        let store = ArtifactStore::new(temp.path()).unwrap();

        let run_id = RunId::new();
        let artifact1 = Artifact::new(
            run_id,
            ArtifactKind::UserPrompt,
            vec![],
            PathBuf::from("a.bin"),
        );
        let artifact2 = Artifact::new(
            run_id,
            ArtifactKind::NormalizedSpec,
            vec![artifact1.id],
            PathBuf::from("b.bin"),
        );

        store.save(&artifact1, b"content1").unwrap();
        store.save(&artifact2, b"content2").unwrap();

        let list = store.list(run_id).unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_delete_run() {
        let temp = TempDir::new().unwrap();
        let store = ArtifactStore::new(temp.path()).unwrap();

        let run_id = RunId::new();
        let artifact = Artifact::new(
            run_id,
            ArtifactKind::UserPrompt,
            vec![],
            PathBuf::from("test.bin"),
        );

        store.save(&artifact, b"content").unwrap();
        assert!(store.exists(artifact.id, run_id));

        store.delete_run(run_id).unwrap();
        assert!(!store.exists(artifact.id, run_id));
    }

    #[test]
    fn test_load_nonexistent() {
        let temp = TempDir::new().unwrap();
        let store = ArtifactStore::new(temp.path()).unwrap();

        let result = store.load(ArtifactId::new(), RunId::new());
        assert!(result.is_err());
    }

    #[test]
    fn test_save_and_load_finding() {
        let temp = TempDir::new().unwrap();
        let store = FindingStore::new(temp.path()).unwrap();

        let run_id = RunId::new();
        let finding = crate::finding::Finding::new(
            run_id,
            "test source".to_string(),
            crate::finding::Severity::High,
            "test category".to_string(),
            vec!["evidence 1".to_string()],
            vec![PathBuf::from("/path/to/file")],
        );

        store.save(&finding).unwrap();

        let loaded = store.load(finding.id, run_id).unwrap();
        assert_eq!(loaded.id, finding.id);
        assert_eq!(loaded.source, finding.source);
        assert_eq!(loaded.severity, finding.severity);
    }

    #[test]
    fn test_list_findings() {
        let temp = TempDir::new().unwrap();
        let store = FindingStore::new(temp.path()).unwrap();

        let run_id = RunId::new();
        let finding1 = crate::finding::Finding::new(
            run_id,
            "source 1".to_string(),
            crate::finding::Severity::Low,
            "category 1".to_string(),
            vec![],
            vec![],
        );
        let finding2 = crate::finding::Finding::new(
            run_id,
            "source 2".to_string(),
            crate::finding::Severity::High,
            "category 2".to_string(),
            vec![],
            vec![],
        );

        store.save(&finding1).unwrap();
        store.save(&finding2).unwrap();

        let list = store.list(run_id).unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_list_by_status() {
        let temp = TempDir::new().unwrap();
        let store = FindingStore::new(temp.path()).unwrap();

        let run_id = RunId::new();
        let mut finding1 = crate::finding::Finding::new(
            run_id,
            "source 1".to_string(),
            crate::finding::Severity::Low,
            "category 1".to_string(),
            vec![],
            vec![],
        );
        finding1.resolve();

        let finding2 = crate::finding::Finding::new(
            run_id,
            "source 2".to_string(),
            crate::finding::Severity::High,
            "category 2".to_string(),
            vec![],
            vec![],
        );

        store.save(&finding1).unwrap();
        store.save(&finding2).unwrap();

        let open = store
            .list_by_status(run_id, crate::finding::FindingStatus::Open)
            .unwrap();
        assert_eq!(open.len(), 1);

        let resolved = store
            .list_by_status(run_id, crate::finding::FindingStatus::Resolved)
            .unwrap();
        assert_eq!(resolved.len(), 1);
    }

    #[test]
    fn test_update_status() {
        let temp = TempDir::new().unwrap();
        let store = FindingStore::new(temp.path()).unwrap();

        let run_id = RunId::new();
        let finding = crate::finding::Finding::new(
            run_id,
            "source".to_string(),
            crate::finding::Severity::Medium,
            "category".to_string(),
            vec![],
            vec![],
        );

        store.save(&finding).unwrap();
        assert_eq!(finding.status, crate::finding::FindingStatus::Open);

        store
            .update_status(finding.id, run_id, crate::finding::FindingStatus::Resolved)
            .unwrap();

        let loaded = store.load(finding.id, run_id).unwrap();
        assert_eq!(loaded.status, crate::finding::FindingStatus::Resolved);
        assert!(loaded.resolved_at.is_some());
    }

    #[test]
    fn test_delete_finding() {
        let temp = TempDir::new().unwrap();
        let store = FindingStore::new(temp.path()).unwrap();

        let run_id = RunId::new();
        let finding = crate::finding::Finding::new(
            run_id,
            "source".to_string(),
            crate::finding::Severity::Low,
            "category".to_string(),
            vec![],
            vec![],
        );

        store.save(&finding).unwrap();
        store.delete(finding.id, run_id).unwrap();

        let result = store.load(finding.id, run_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_run_findings() {
        let temp = TempDir::new().unwrap();
        let store = FindingStore::new(temp.path()).unwrap();

        let run_id = RunId::new();
        let finding = crate::finding::Finding::new(
            run_id,
            "source".to_string(),
            crate::finding::Severity::Low,
            "category".to_string(),
            vec![],
            vec![],
        );

        store.save(&finding).unwrap();
        store.delete_run(run_id).unwrap();

        let list = store.list(run_id).unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn test_load_nonexistent_finding() {
        let temp = TempDir::new().unwrap();
        let store = FindingStore::new(temp.path()).unwrap();

        let result = store.load(crate::finding::FindingId::new(), RunId::new());
        assert!(result.is_err());
    }

    #[test]
    fn test_list_findings_nonexistent_run() {
        let temp = TempDir::new().unwrap();
        let store = FindingStore::new(temp.path()).unwrap();

        let list = store.list(RunId::new()).unwrap();
        assert!(list.is_empty());
    }
}
