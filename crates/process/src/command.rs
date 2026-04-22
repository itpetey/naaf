use std::{
    ffi::OsString,
    path::PathBuf,
    process::{ExitStatus, Output},
};

use tokio::process::Command;

use crate::error::ProcessError;

/// A process invocation that can be executed by `ProcessTask`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessCommand {
    program: OsString,
    args: Vec<OsString>,
    current_dir: Option<PathBuf>,
    env: Vec<(OsString, OsString)>,
}

/// The collected process output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessOutput {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl ProcessCommand {
    /// Creates a command that invokes a program directly.
    pub fn new(program: impl Into<OsString>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            current_dir: None,
            env: Vec::new(),
        }
    }

    /// Creates a command that runs inside the system shell.
    pub fn shell(script: impl Into<OsString>) -> Self {
        Self::new("sh").arg("-lc").arg(script)
    }

    /// Adds one argument.
    pub fn arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Adds multiple arguments.
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Sets the working directory.
    pub fn current_dir(mut self, current_dir: impl Into<PathBuf>) -> Self {
        self.current_dir = Some(current_dir.into());
        self
    }

    /// Sets one environment variable.
    pub fn env(mut self, key: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    /// Sets multiple environment variables.
    pub fn envs<I, K, V>(mut self, envs: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<OsString>,
        V: Into<OsString>,
    {
        self.env.extend(
            envs.into_iter()
                .map(|(key, value)| (key.into(), value.into())),
        );
        self
    }

    pub(crate) async fn execute(&self) -> Result<ProcessOutput, ProcessError> {
        let mut command = Command::new(&self.program);
        command.args(&self.args);

        if let Some(current_dir) = &self.current_dir {
            command.current_dir(current_dir);
        }

        command.envs(self.env.iter().map(|(key, value)| (key, value)));

        let output = command.output().await.map_err(ProcessError::Io)?;
        let output = ProcessOutput::from(output);

        if output.status.success() {
            Ok(output)
        } else {
            Err(ProcessError::Exit {
                status: output.status,
                stdout: output.stdout,
                stderr: output.stderr,
            })
        }
    }
}

impl From<Output> for ProcessOutput {
    fn from(output: Output) -> Self {
        Self {
            status: output.status,
            stdout: output.stdout,
            stderr: output.stderr,
        }
    }
}
