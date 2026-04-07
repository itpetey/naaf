use chrono::{DateTime, Utc};
use naaf_schema::state::{RunId, StateId};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;
use thiserror::Error;

/// Errors that can occur during event emission or persistence.
#[derive(Debug, Error)]
pub enum EventError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Lock poisoned")]
    LockPoisoned,
}

/// Result type for event operations.
pub type EventResult = Result<(), EventError>;

/// Events emitted during workflow execution for tracing and debugging.
///
/// Each event includes:
/// - `run_id`: Unique identifier for the workflow run
/// - `state_id`: Identifier for the state at the time of the event
/// - `step_name`: Name of the step that triggered the event
/// - `sequence_number`: Monotonically increasing sequence number for ordering
/// - `timestamp`: When the event occurred
///
/// # Example
/// ```rust,ignore
/// use naaf_core::events::ExecutionEvent;
/// use naaf_schema::state::{RunId, StateId};
/// use chrono::Utc;
///
/// let event = ExecutionEvent::RunStarted {
/// run_id: RunId::new(),
/// state_id: StateId::new(),
/// step_name: "start".to_string(),
/// sequence_number: 0,
/// timestamp: Utc::now(),
/// };
/// ```

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum ExecutionEvent {
    RunStarted {
        run_id: RunId,
        state_id: StateId,
        step_name: String,
        sequence_number: u64,
        timestamp: DateTime<Utc>,
    },
    StepEntered {
        run_id: RunId,
        state_id: StateId,
        step_name: String,
        sequence_number: u64,
        timestamp: DateTime<Utc>,
    },
    PromptRendered {
        run_id: RunId,
        state_id: StateId,
        step_name: String,
        sequence_number: u64,
        timestamp: DateTime<Utc>,
    },
    ProviderCalled {
        run_id: RunId,
        state_id: StateId,
        step_name: String,
        sequence_number: u64,
        timestamp: DateTime<Utc>,
    },
    ProviderResponded {
        run_id: RunId,
        state_id: StateId,
        step_name: String,
        sequence_number: u64,
        timestamp: DateTime<Utc>,
    },
    ArtifactsParsed {
        run_id: RunId,
        state_id: StateId,
        step_name: String,
        sequence_number: u64,
        timestamp: DateTime<Utc>,
    },
    ValidatorPassed {
        run_id: RunId,
        state_id: StateId,
        step_name: String,
        sequence_number: u64,
        timestamp: DateTime<Utc>,
    },
    ValidatorFailed {
        run_id: RunId,
        state_id: StateId,
        step_name: String,
        sequence_number: u64,
        timestamp: DateTime<Utc>,
    },
    RouteSelected {
        run_id: RunId,
        state_id: StateId,
        step_name: String,
        sequence_number: u64,
        timestamp: DateTime<Utc>,
    },
    BranchStarted {
        run_id: RunId,
        state_id: StateId,
        step_name: String,
        sequence_number: u64,
        timestamp: DateTime<Utc>,
    },
    BranchCompleted {
        run_id: RunId,
        state_id: StateId,
        step_name: String,
        sequence_number: u64,
        timestamp: DateTime<Utc>,
    },
    JoinReduced {
        run_id: RunId,
        state_id: StateId,
        step_name: String,
        sequence_number: u64,
        timestamp: DateTime<Utc>,
    },
    RunTerminated {
        run_id: RunId,
        state_id: StateId,
        step_name: String,
        sequence_number: u64,
        timestamp: DateTime<Utc>,
    },
    RunFailed {
        run_id: RunId,
        state_id: StateId,
        step_name: String,
        error: String,
        sequence_number: u64,
        timestamp: DateTime<Utc>,
    },
}

/// Synchronous trait for emitting execution events.
///
/// Implementations can persist events to various backends (filesystem, database, etc.)
/// or simply ignore them (e.g., `NoOpTraceSink` for testing).
///
/// # Example
/// ```rust,ignore
/// use naaf_core::events::{TraceSink, ExecutionEvent};
///
/// struct MyTraceSink;
///
/// impl TraceSink for MyTraceSink {
/// fn emit(&self, event: ExecutionEvent) -> EventResult {
/// println!("Event: {:?}", event);
/// Ok(())
/// }
/// }
/// ```
pub trait TraceSink: Send + Sync {
    fn emit(&self, event: ExecutionEvent) -> EventResult;
}

impl<T: TraceSink + ?Sized> TraceSink for Box<T> {
    fn emit(&self, event: ExecutionEvent) -> EventResult {
        (**self).emit(event)
    }
}

/// No-op trace sink for testing and scenarios where event tracing is not needed.
#[derive(Clone, Default)]
pub struct NoOpTraceSink;

impl TraceSink for NoOpTraceSink {
    fn emit(&self, _event: ExecutionEvent) -> EventResult {
        Ok(())
    }
}

/// Asynchronous variant of TraceSink for async backends.
pub trait AsyncTraceSink: Send + Sync {
    fn emit(&self, event: ExecutionEvent) -> impl std::future::Future<Output = EventResult> + Send;
}

/// No-op async trace sink for testing.
#[derive(Clone, Default)]
pub struct NoOpAsyncTraceSink;

impl AsyncTraceSink for NoOpAsyncTraceSink {
    async fn emit(&self, _event: ExecutionEvent) -> EventResult {
        Ok(())
    }
}

/// Trait for persisting events to storage.
pub trait EventStore: Send + Sync {
    fn store(&self, event: &ExecutionEvent) -> Result<(), std::io::Error>;
}

/// Filesystem-based event store that writes events as JSON lines.
///
/// Events are appended to a filein JSON format, one per line.
/// This format is simple to parse and supports concurrent writes via locks.
///
/// # Example
/// ```rust,ignore
/// use naaf_core::events::FilesystemEventStore;
/// use std::path::PathBuf;
///
/// let store = FilesystemEventStore::new(&PathBuf::from("events.log"))?;
/// store.store(&event)?; // Writes event as JSON line
/// ```
pub struct FilesystemEventStore {
    file: Mutex<std::fs::File>,
}

impl FilesystemEventStore {
    pub fn new(path: &Path) -> Result<Self, std::io::Error> {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        Ok(Self {
            file: Mutex::new(file),
        })
    }
}

impl EventStore for FilesystemEventStore {
    fn store(&self, event: &ExecutionEvent) -> Result<(), std::io::Error> {
        let mut file = self
            .file
            .lock()
            .map_err(|_| std::io::Error::other("Failed to lock file"))?;
        let json = serde_json::to_string(event)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        writeln!(file, "{}", json)
    }
}

impl TraceSink for FilesystemEventStore {
    fn emit(&self, event: ExecutionEvent) -> EventResult {
        self.store(&event).map_err(EventError::from)
    }
}

impl FilesystemEventStore {
    /// Read all events from a file.
    ///
    /// Returns events in the order they were written.
    /// Events that fail to deserialize are skipped.
    ///
    /// # Example
    /// ```rust,ignore
    /// use naaf_core::events::FilesystemEventStore;
    /// use std::path::Path;
    ///
    /// let events = FilesystemEventStore::read_events(Path::new("events.log"))?;
    /// for event in events {
    /// println!("{:?}", event);
    /// }
    /// ```
    pub fn read_events(path: &Path) -> Result<Vec<ExecutionEvent>, std::io::Error> {
        use std::fs::File;
        use std::io::{BufRead, BufReader};

        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut events = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if let Ok(event) = serde_json::from_str::<ExecutionEvent>(&line) {
                events.push(event);
            }
        }

        Ok(events)
    }

    /// Read events from a file, filtering by run_id.
    ///
    /// # Example
    /// ```rust,ignore
    /// use naaf_core::events::FilesystemEventStore;
    /// use naaf_schema::state::RunId;
    /// use std::path::Path;
    ///
    /// let run_id = RunId::new();
    /// let events = FilesystemEventStore::read_events_by_run(Path::new("events.log"), run_id)?;
    /// ```
    pub fn read_events_by_run(
        path: &Path,
        run_id: RunId,
    ) -> Result<Vec<ExecutionEvent>, std::io::Error> {
        let all_events = Self::read_events(path)?;
        Ok(all_events
            .into_iter()
            .filter(|event| match event {
                ExecutionEvent::RunStarted { run_id: id, .. } => *id == run_id,
                ExecutionEvent::StepEntered { run_id: id, .. } => *id == run_id,
                ExecutionEvent::PromptRendered { run_id: id, .. } => *id == run_id,
                ExecutionEvent::ProviderCalled { run_id: id, .. } => *id == run_id,
                ExecutionEvent::ProviderResponded { run_id: id, .. } => *id == run_id,
                ExecutionEvent::ArtifactsParsed { run_id: id, .. } => *id == run_id,
                ExecutionEvent::ValidatorPassed { run_id: id, .. } => *id == run_id,
                ExecutionEvent::ValidatorFailed { run_id: id, .. } => *id == run_id,
                ExecutionEvent::RouteSelected { run_id: id, .. } => *id == run_id,
                ExecutionEvent::BranchStarted { run_id: id, .. } => *id == run_id,
                ExecutionEvent::BranchCompleted { run_id: id, .. } => *id == run_id,
                ExecutionEvent::JoinReduced { run_id: id, .. } => *id == run_id,
                ExecutionEvent::RunTerminated { run_id: id, .. } => *id == run_id,
                ExecutionEvent::RunFailed { run_id: id, .. } => *id == run_id,
            })
            .collect())
    }
}
