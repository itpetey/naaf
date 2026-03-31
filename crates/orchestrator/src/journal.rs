//! Append-only JSONL run journal.

use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, LineWriter, Write};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::run::{Phase, RunId, TaskId};

const JOURNAL_FILE: &str = "journal.jsonl";

#[derive(Debug, Error)]
pub enum JournalError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Run not found: {0:?}")]
    RunNotFound(RunId),
}

pub type JournalResult<T> = Result<T, JournalError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    TaskCreated {
        task_id: TaskId,
        prompt: String,
        timestamp: DateTime<Utc>,
    },

    RunCreated {
        run_id: RunId,
        task_id: TaskId,
        timestamp: DateTime<Utc>,
    },

    RunStarted {
        run_id: RunId,
        timestamp: DateTime<Utc>,
    },

    ReviewStarted {
        run_id: RunId,
        timestamp: DateTime<Utc>,
    },

    TransitionExecuted {
        run_id: RunId,
        from_phase: Phase,
        to_phase: Phase,
        worker_id: String,
        artifact_id: Option<crate::artifact::ArtifactId>,
        timestamp: DateTime<Utc>,
    },

    ArtifactCreated {
        run_id: RunId,
        artifact_id: crate::artifact::ArtifactId,
        kind: crate::artifact::ArtifactKind,
        parent_ids: Vec<crate::artifact::ArtifactId>,
        timestamp: DateTime<Utc>,
    },

    FindingCreated {
        run_id: RunId,
        finding_id: crate::finding::FindingId,
        severity: crate::finding::Severity,
        category: String,
        timestamp: DateTime<Utc>,
    },

    FindingResolved {
        run_id: RunId,
        finding_id: crate::finding::FindingId,
        timestamp: DateTime<Utc>,
    },

    RunCompleted {
        run_id: RunId,
        timestamp: DateTime<Utc>,
    },

    RunFailed {
        run_id: RunId,
        reason: String,
        timestamp: DateTime<Utc>,
    },

    RunEscalated {
        run_id: RunId,
        reason: String,
        timestamp: DateTime<Utc>,
    },
}

impl Event {
    pub fn run_id(&self) -> Option<RunId> {
        match self {
            Event::TaskCreated { .. } => None,
            Event::RunCreated { run_id, .. } => Some(*run_id),
            Event::RunStarted { run_id, .. } => Some(*run_id),
            Event::ReviewStarted { run_id, .. } => Some(*run_id),
            Event::TransitionExecuted { run_id, .. } => Some(*run_id),
            Event::ArtifactCreated { run_id, .. } => Some(*run_id),
            Event::FindingCreated { run_id, .. } => Some(*run_id),
            Event::FindingResolved { run_id, .. } => Some(*run_id),
            Event::RunCompleted { run_id, .. } => Some(*run_id),
            Event::RunFailed { run_id, .. } => Some(*run_id),
            Event::RunEscalated { run_id, .. } => Some(*run_id),
        }
    }
}

pub struct Journal {
    path: PathBuf,
}

impl Journal {
    pub fn new(root: impl Into<PathBuf>) -> JournalResult<Self> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        let path = root.join(JOURNAL_FILE);
        Ok(Self { path })
    }

    pub fn root(&self) -> &Path {
        &self.path
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
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        Ok(JournalIter {
            reader,
            current: None,
        })
    }

    pub fn for_run(&self, run_id: RunId) -> JournalResult<RunJournalIter> {
        Ok(RunJournalIter {
            inner: self.iter()?,
            run_id,
        })
    }

    pub fn latest_for_run(&self, run_id: RunId) -> JournalResult<Option<Event>> {
        let mut latest: Option<Event> = None;
        for event in self.for_run(run_id)? {
            latest = Some(event?);
        }
        Ok(latest)
    }
}

pub struct JournalIter {
    reader: BufReader<File>,
    current: Option<Event>,
}

impl Iterator for JournalIter {
    type Item = JournalResult<Event>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut line = String::new();
        match self.reader.read_line(&mut line) {
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

pub struct RunJournalIter {
    inner: JournalIter,
    run_id: RunId,
}

impl Iterator for RunJournalIter {
    type Item = JournalResult<Event>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let event = self.inner.next()?;
            match event {
                Ok(e) if e.run_id() == Some(self.run_id) => return Some(Ok(e)),
                Ok(_) => continue,
                Err(e) => return Some(Err(e)),
            }
        }
    }
}

pub fn task_created(task_id: TaskId, prompt: &str) -> Event {
    Event::TaskCreated {
        task_id,
        prompt: prompt.to_string(),
        timestamp: Utc::now(),
    }
}

pub fn run_created(run_id: RunId, task_id: TaskId) -> Event {
    Event::RunCreated {
        run_id,
        task_id,
        timestamp: Utc::now(),
    }
}

pub fn run_started(run_id: RunId) -> Event {
    Event::RunStarted {
        run_id,
        timestamp: Utc::now(),
    }
}

pub fn transition_executed(
    run_id: RunId,
    from_phase: Phase,
    to_phase: Phase,
    worker_id: &str,
    artifact_id: Option<crate::artifact::ArtifactId>,
) -> Event {
    Event::TransitionExecuted {
        run_id,
        from_phase,
        to_phase,
        worker_id: worker_id.to_string(),
        artifact_id,
        timestamp: Utc::now(),
    }
}

pub fn artifact_created(
    run_id: RunId,
    artifact_id: crate::artifact::ArtifactId,
    kind: crate::artifact::ArtifactKind,
    parent_ids: Vec<crate::artifact::ArtifactId>,
) -> Event {
    Event::ArtifactCreated {
        run_id,
        artifact_id,
        kind,
        parent_ids,
        timestamp: Utc::now(),
    }
}

pub fn finding_created(
    run_id: RunId,
    finding_id: crate::finding::FindingId,
    severity: crate::finding::Severity,
    category: &str,
) -> Event {
    Event::FindingCreated {
        run_id,
        finding_id,
        severity,
        category: category.to_string(),
        timestamp: Utc::now(),
    }
}

pub fn finding_resolved(run_id: RunId, finding_id: crate::finding::FindingId) -> Event {
    Event::FindingResolved {
        run_id,
        finding_id,
        timestamp: Utc::now(),
    }
}

pub fn run_completed(run_id: RunId) -> Event {
    Event::RunCompleted {
        run_id,
        timestamp: Utc::now(),
    }
}

pub fn run_failed(run_id: RunId, reason: &str) -> Event {
    Event::RunFailed {
        run_id,
        reason: reason.to_string(),
        timestamp: Utc::now(),
    }
}

pub fn run_escalated(run_id: RunId, reason: &str) -> Event {
    Event::RunEscalated {
        run_id,
        reason: reason.to_string(),
        timestamp: Utc::now(),
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_append_and_iter() {
        let temp = TempDir::new().unwrap();
        let journal = Journal::new(temp.path()).unwrap();

        let task_id = TaskId::new();
        let run_id = RunId::new();

        let event1 = task_created(task_id, "test prompt");
        let event2 = run_created(run_id, task_id);
        let event3 = run_started(run_id);

        journal.append(&event1).unwrap();
        journal.append(&event2).unwrap();
        journal.append(&event3).unwrap();

        let events: Vec<_> = journal.iter().unwrap().map(|e| e.unwrap()).collect();
        assert_eq!(events.len(), 3);
    }

    #[test]
    fn test_for_run_filter() {
        let temp = TempDir::new().unwrap();
        let journal = Journal::new(temp.path()).unwrap();

        let task_id = TaskId::new();
        let run_id1 = RunId::new();
        let run_id2 = RunId::new();

        journal.append(&task_created(task_id, "prompt")).unwrap();
        journal.append(&run_created(run_id1, task_id)).unwrap();
        journal.append(&run_started(run_id1)).unwrap();
        journal.append(&run_created(run_id2, task_id)).unwrap();
        journal.append(&run_started(run_id2)).unwrap();

        let run1_events: Vec<_> = journal
            .for_run(run_id1)
            .unwrap()
            .map(|e| e.unwrap())
            .collect();
        assert_eq!(run1_events.len(), 2);
    }

    #[test]
    fn test_event_run_id() {
        let task_id = TaskId::new();
        let run_id = RunId::new();

        let task_event = task_created(task_id, "prompt");
        assert_eq!(task_event.run_id(), None);

        let run_event = run_created(run_id, task_id);
        assert_eq!(run_event.run_id(), Some(run_id));
    }

    #[test]
    fn test_transition_executed_json_format() {
        let run_id = RunId::new();
        let artifact_id = crate::artifact::ArtifactId::new();

        let event = transition_executed(
            run_id,
            Phase::Proposed,
            Phase::Normalized,
            "request_normalizer",
            Some(artifact_id),
        );

        let json = serde_json::to_string(&event).unwrap();

        assert!(json.contains(r#""type":"transition_executed""#));
        assert!(json.contains(&format!(r#""run_id":"{}""#, run_id)));
        assert!(json.contains(r#""from_phase":"Proposed"#));
        assert!(json.contains(r#""to_phase":"Normalized"#));
        assert!(json.contains(r#""worker_id":"request_normalizer""#));
        assert!(json.contains(&format!(r#""artifact_id":"{}""#, artifact_id)));

        let decoded: Event = serde_json::from_str(&json).unwrap();
        match decoded {
            Event::TransitionExecuted {
                run_id: decoded_run_id,
                from_phase,
                to_phase,
                worker_id,
                artifact_id: decoded_artifact_id,
                ..
            } => {
                assert_eq!(decoded_run_id, run_id);
                assert_eq!(from_phase, Phase::Proposed);
                assert_eq!(to_phase, Phase::Normalized);
                assert_eq!(worker_id, "request_normalizer");
                assert_eq!(decoded_artifact_id, Some(artifact_id));
            }
            _ => panic!("Expected TransitionExecuted event"),
        }
    }
}
