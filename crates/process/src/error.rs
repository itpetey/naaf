use std::process::ExitStatus;

use thiserror::Error;

/// Errors raised by the generic `naaf_core::Check` adapter.
pub type CheckError<BuildError, DecodeError> = AdapterError<BuildError, DecodeError>;
/// Errors raised by the generic `naaf_core::Materialiser` adapter.
pub type MaterialiserError<BuildError, DecodeError> = AdapterError<BuildError, DecodeError>;
/// Errors raised by the generic `naaf_core::RepairPlanner` adapter.
pub type RepairPlannerError<BuildError, DecodeError> = AdapterError<BuildError, DecodeError>;
/// Errors raised by the generic `naaf_core::Task` adapter.
pub type TaskError<BuildError, DecodeError> = AdapterError<BuildError, DecodeError>;

/// Errors raised while executing a process.
#[derive(Debug, Error)]
pub enum ProcessError {
    /// Spawning or waiting on the process failed.
    #[error("process execution failed: {0}")]
    Io(#[source] std::io::Error),
    /// The process exited with a non-zero status.
    #[error("process exited unsuccessfully: {status}")]
    Exit {
        status: ExitStatus,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
    },
}

/// Errors raised by the generic `naaf_core` role adapters.
#[derive(Debug, Error)]
pub enum AdapterError<BuildError, DecodeError> {
    /// Building the process invocation failed.
    #[error("failed to build process command: {0}")]
    Build(#[source] BuildError),
    /// Running the process failed.
    #[error(transparent)]
    Execute(#[from] ProcessError),
    /// Decoding the process output failed.
    #[error("failed to decode process output: {0}")]
    Decode(#[source] DecodeError),
}
