//! Append-only JSONL run journal.
//!
//! # Legacy Code
//!
//! This module is part of the legacy prototype runtime.
//! **Do not build new features on this code.**
//! See the repository root `LEGACY.md` for details.

use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, LineWriter, Write};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::artifact::ArtifactId;
use crate::run::Phase;

const JOURNAL_FILE: &str = "journal.jsonl";

#[derive(Debug, Error)]
pub enum JournalError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type JournalResult<T> = Result<T, JournalError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    RunStarted {
        timestamp: DateTime<Utc>,
    },

    ReviewStarted {
        timestamp: DateTime<Utc>,
    },

    TransitionExecuted {
        from_phase: Phase,
        to_phase: Phase,
        worker_id: String,
        artifact_id: Option<ArtifactId>,
        timestamp: DateTime<Utc>,
    },

    ArtifactCreated {
        artifact_id: ArtifactId,
        kind: crate::artifact::ArtifactKind,
        parent_ids: Vec<ArtifactId>,
        timestamp: DateTime<Utc>,
    },

    FindingCreated {
        finding_id: crate::finding::FindingId,
        severity: crate::finding::Severity,
        category: String,
        timestamp: DateTime<Utc>,
    },

    FindingResolved {
        finding_id: crate::finding::FindingId,
        timestamp: DateTime<Utc>,
    },

    RunCompleted {
        timestamp: DateTime<Utc>,
    },

    RunFailed {
        reason: String,
        timestamp: DateTime<Utc>,
    },

    RunEscalated {
        reason: String,
        timestamp: DateTime<Utc>,
    },
}

pub struct Journal {
    path: PathBuf,
}

impl Journal {
    pub fn new(run_dir: impl Into<PathBuf>) -> JournalResult<Self> {
        let run_dir = run_dir.into();
        fs::create_dir_all(&run_dir)?;
        let path = run_dir.join(JOURNAL_FILE);
        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn append(&self, event: &Event) -> JournalResult<()> {
        let file = BufWriter::new(
            fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)?,
        );
        let mut writer = LineWriter::new(file);
        serde_json::to_writer(&mut writer, event)?;
        writer.write_all(b"\n")?;
        writer.flush()?;
        Ok(())
    }

    pub fn iter(&self) -> JournalResult<JournalIter> {
        if !self.path.exists() {
            return Ok(JournalIter::empty());
        }
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        Ok(JournalIter::from_reader(reader))
    }

    pub fn latest(&self) -> JournalResult<Option<Event>> {
        let mut latest: Option<Event> = None;
        for event in self.iter()? {
            latest = Some(event?);
        }
        Ok(latest)
    }
}

pub struct JournalIter {
    reader: Option<BufReader<File>>,
    current: Option<Event>,
}

impl JournalIter {
    fn from_reader(reader: BufReader<File>) -> Self {
        Self {
            reader: Some(reader),
            current: None,
        }
    }

    fn empty() -> Self {
        Self {
            reader: None,
            current: None,
        }
    }
}

impl Iterator for JournalIter {
    type Item = JournalResult<Event>;

    fn next(&mut self) -> Option<Self::Item> {
        let reader = self.reader.as_mut()?;
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => None,
            Ok(_) => {
                let event: Event = match serde_json::from_str(&line) {
                    Ok(e) => e,
                    Err(e) => return Some(Err(JournalError::Serialization(e))),
                };
                self.current = Some(event);
                Some(Ok(self.current.as_ref().unwrap().clone()))
            }
            Err(e) => Some(Err(JournalError::Io(e))),
        }
    }
}

pub fn run_started() -> Event {
    Event::RunStarted {
        timestamp: Utc::now(),
    }
}

pub fn review_started() -> Event {
    Event::ReviewStarted {
        timestamp: Utc::now(),
    }
}

pub fn transition_executed(
    from_phase: Phase,
    to_phase: Phase,
    worker_id: &str,
    artifact_id: Option<ArtifactId>,
) -> Event {
    Event::TransitionExecuted {
        from_phase,
        to_phase,
        worker_id: worker_id.to_string(),
        artifact_id,
        timestamp: Utc::now(),
    }
}

pub fn artifact_created(
    artifact_id: ArtifactId,
    kind: crate::artifact::ArtifactKind,
    parent_ids: Vec<ArtifactId>,
) -> Event {
    Event::ArtifactCreated {
        artifact_id,
        kind,
        parent_ids,
        timestamp: Utc::now(),
    }
}

pub fn finding_created(
    finding_id: crate::finding::FindingId,
    severity: crate::finding::Severity,
    category: &str,
) -> Event {
    Event::FindingCreated {
        finding_id,
        severity,
        category: category.to_string(),
        timestamp: Utc::now(),
    }
}

pub fn finding_resolved(finding_id: crate::finding::FindingId) -> Event {
    Event::FindingResolved {
        finding_id,
        timestamp: Utc::now(),
    }
}

pub fn run_completed() -> Event {
    Event::RunCompleted {
        timestamp: Utc::now(),
    }
}

pub fn run_failed(reason: &str) -> Event {
    Event::RunFailed {
        reason: reason.to_string(),
        timestamp: Utc::now(),
    }
}

pub fn run_escalated(reason: &str) -> Event {
    Event::RunEscalated {
        reason: reason.to_string(),
        timestamp: Utc::now(),
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::artifact::ArtifactKind;

    #[test]
    fn test_append_and_iter() {
        let temp = TempDir::new().unwrap();
        let journal = Journal::new(temp.path()).unwrap();

        let event1 = run_started();
        let event2 = review_started();
        let event3 = run_completed();

        journal.append(&event1).unwrap();
        journal.append(&event2).unwrap();
        journal.append(&event3).unwrap();

        let events: Vec<_> = journal.iter().unwrap().map(|e| e.unwrap()).collect();
        assert_eq!(events.len(), 3);
    }

    #[test]
    fn test_latest() {
        let temp = TempDir::new().unwrap();
        let journal = Journal::new(temp.path()).unwrap();

        journal.append(&run_started()).unwrap();
        journal.append(&run_completed()).unwrap();

        let latest = journal.latest().unwrap().unwrap();
        assert!(matches!(latest, Event::RunCompleted { .. }));
    }

    #[test]
    fn test_transition_executed_json_format() {
        let artifact_id = ArtifactId::new();

        let event = transition_executed(
            Phase::Proposed,
            Phase::Normalized,
            "request_normalizer",
            Some(artifact_id),
        );

        let json = serde_json::to_string(&event).unwrap();

        assert!(json.contains(r#""type":"transition_executed""#));
        assert!(json.contains(r#""from_phase":"Proposed"#));
        assert!(json.contains(r#""to_phase":"Normalized"#));
        assert!(json.contains(r#""worker_id":"request_normalizer""#));
        assert!(json.contains(&format!(r#""artifact_id":"{}""#, artifact_id)));

        let decoded: Event = serde_json::from_str(&json).unwrap();
        match decoded {
            Event::TransitionExecuted {
                from_phase,
                to_phase,
                worker_id,
                artifact_id: decoded_artifact_id,
                ..
            } => {
                assert_eq!(from_phase, Phase::Proposed);
                assert_eq!(to_phase, Phase::Normalized);
                assert_eq!(worker_id, "request_normalizer");
                assert_eq!(decoded_artifact_id, Some(artifact_id));
            }
            _ => panic!("Expected TransitionExecuted event"),
        }
    }

    #[test]
    fn test_artifact_created() {
        let artifact_id = ArtifactId::new();
        let event = artifact_created(artifact_id, ArtifactKind::UserPrompt, vec![]);

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains(r#""type":"artifact_created""#));

        let decoded: Event = serde_json::from_str(&json).unwrap();
        match decoded {
            Event::ArtifactCreated {
                kind,
                artifact_id: decoded_id,
                ..
            } => {
                assert_eq!(kind, ArtifactKind::UserPrompt);
                assert_eq!(decoded_id, artifact_id);
            }
            _ => panic!("Expected ArtifactCreated event"),
        }
    }
}
