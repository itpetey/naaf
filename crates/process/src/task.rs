use std::{convert::Infallible, marker::PhantomData};

use futures::future::LocalBoxFuture;
use naaf_core::{Attempt, Check, Materialiser, RepairPlanner, Task};

use crate::{AdapterError, ProcessCommand, ProcessOutput};

type AdapterMarker<Input, Output, BuildError, DecodeError> =
    PhantomData<fn(Input) -> Result<Output, (BuildError, DecodeError)>>;
type AdapterFuture<'a, Output, BuildError, DecodeError> =
    LocalBoxFuture<'a, Result<Output, AdapterError<BuildError, DecodeError>>>;
type RepairAttempts<Input, Artefact, Finding> = Vec<Attempt<Input, Artefact, Finding>>;
type RepairAdapter<R, Build, Decode, Input, Artefact, Finding, BuildError, DecodeError> =
    ProcessRoleAdapter<
        R,
        Build,
        Decode,
        RepairAttempts<Input, Artefact, Finding>,
        Input,
        BuildError,
        DecodeError,
    >;

struct ProcessRoleAdapter<R, Build, Decode, Input, Output, BuildError, DecodeError> {
    build_command: Build,
    decode_output: Decode,
    marker: AdapterMarker<Input, Output, BuildError, DecodeError>,
    runtime: PhantomData<fn(&R)>,
}

impl<R, Build, Decode, Input, Output, BuildError, DecodeError>
    ProcessRoleAdapter<R, Build, Decode, Input, Output, BuildError, DecodeError>
{
    fn new(build_command: Build, decode_output: Decode) -> Self {
        Self {
            build_command,
            decode_output,
            marker: PhantomData,
            runtime: PhantomData,
        }
    }
}

impl<R, Build, Decode, Input, Output, BuildError, DecodeError>
    ProcessRoleAdapter<R, Build, Decode, Input, Output, BuildError, DecodeError>
where
    Build: Fn(&R, Input) -> Result<ProcessCommand, BuildError> + 'static,
    Decode: Fn(ProcessOutput) -> Result<Output, DecodeError> + 'static,
{
    fn execute<'a>(
        &'a self,
        runtime: &'a R,
        input: Input,
    ) -> AdapterFuture<'a, Output, BuildError, DecodeError> {
        Box::pin(async move {
            let command = (self.build_command)(runtime, input).map_err(AdapterError::Build)?;
            let output = command.execute().await.map_err(AdapterError::Execute)?;
            (self.decode_output)(output).map_err(AdapterError::Decode)
        })
    }
}

/// A generic `naaf_core::Task` backed by a local process.
pub struct ProcessTask<R, Build, Decode, Input, Output, BuildError, DecodeError> {
    adapter: ProcessRoleAdapter<R, Build, Decode, Input, Output, BuildError, DecodeError>,
}

impl<R, Build, Decode, Input, Output, DecodeError>
    ProcessTask<R, Build, Decode, Input, Output, Infallible, DecodeError>
{
    /// Creates a process task that cannot fail while building the command.
    pub fn new(build_command: Build, decode_output: Decode) -> Self {
        Self::with_builder(build_command, decode_output)
    }
}

impl<R, Build, Decode, Input, Output, BuildError, DecodeError>
    ProcessTask<R, Build, Decode, Input, Output, BuildError, DecodeError>
{
    /// Creates a process task with an explicit build error type.
    pub fn with_builder(build_command: Build, decode_output: Decode) -> Self {
        Self {
            adapter: ProcessRoleAdapter::new(build_command, decode_output),
        }
    }
}

impl<R, Build, Decode, Input, Output, BuildError, DecodeError> Task
    for ProcessTask<R, Build, Decode, Input, Output, BuildError, DecodeError>
where
    R: 'static,
    Input: 'static,
    Output: 'static,
    BuildError: 'static,
    DecodeError: 'static,
    Build: Fn(&R, Input) -> Result<ProcessCommand, BuildError> + 'static,
    Decode: Fn(ProcessOutput) -> Result<Output, DecodeError> + 'static,
{
    type Runtime = R;
    type Input = Input;
    type Output = Output;
    type Error = AdapterError<BuildError, DecodeError>;

    fn run<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        input: Self::Input,
    ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
        self.adapter.execute(runtime, input)
    }
}

/// A generic `naaf_core::Check` backed by a local process.
pub struct ProcessCheck<R, Build, Decode, Subject, Finding, BuildError, DecodeError> {
    adapter: ProcessRoleAdapter<R, Build, Decode, Subject, Vec<Finding>, BuildError, DecodeError>,
}

impl<R, Build, Decode, Subject, Finding, DecodeError>
    ProcessCheck<R, Build, Decode, Subject, Finding, Infallible, DecodeError>
{
    /// Creates a process check that cannot fail while building the command.
    pub fn new(build_command: Build, decode_findings: Decode) -> Self {
        Self::with_builder(build_command, decode_findings)
    }
}

impl<R, Build, Decode, Subject, Finding, BuildError, DecodeError>
    ProcessCheck<R, Build, Decode, Subject, Finding, BuildError, DecodeError>
{
    /// Creates a process check with an explicit build error type.
    pub fn with_builder(build_command: Build, decode_findings: Decode) -> Self {
        Self {
            adapter: ProcessRoleAdapter::new(build_command, decode_findings),
        }
    }
}

impl<R, Build, Decode, Subject, Finding, BuildError, DecodeError> Check
    for ProcessCheck<R, Build, Decode, Subject, Finding, BuildError, DecodeError>
where
    R: 'static,
    Subject: 'static,
    Finding: 'static,
    BuildError: 'static,
    DecodeError: 'static,
    Build: Fn(&R, Subject) -> Result<ProcessCommand, BuildError> + 'static,
    Decode: Fn(ProcessOutput) -> Result<Vec<Finding>, DecodeError> + 'static,
{
    type Runtime = R;
    type Subject = Subject;
    type Finding = Finding;
    type Error = AdapterError<BuildError, DecodeError>;

    fn check<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        subject: Self::Subject,
    ) -> LocalBoxFuture<'a, Result<Vec<Self::Finding>, Self::Error>> {
        self.adapter.execute(runtime, subject)
    }
}

/// A generic `naaf_core::Materialiser` backed by a local process.
pub struct ProcessMaterialiser<R, Build, Decode, Input, Output, BuildError, DecodeError> {
    adapter: ProcessRoleAdapter<R, Build, Decode, Input, Output, BuildError, DecodeError>,
}

impl<R, Build, Decode, Input, Output, DecodeError>
    ProcessMaterialiser<R, Build, Decode, Input, Output, Infallible, DecodeError>
{
    /// Creates a process materialiser that cannot fail while building the command.
    pub fn new(build_command: Build, decode_output: Decode) -> Self {
        Self::with_builder(build_command, decode_output)
    }
}

impl<R, Build, Decode, Input, Output, BuildError, DecodeError>
    ProcessMaterialiser<R, Build, Decode, Input, Output, BuildError, DecodeError>
{
    /// Creates a process materialiser with an explicit build error type.
    pub fn with_builder(build_command: Build, decode_output: Decode) -> Self {
        Self {
            adapter: ProcessRoleAdapter::new(build_command, decode_output),
        }
    }
}

impl<R, Build, Decode, Input, Output, BuildError, DecodeError> Materialiser
    for ProcessMaterialiser<R, Build, Decode, Input, Output, BuildError, DecodeError>
where
    R: 'static,
    Input: 'static,
    Output: 'static,
    BuildError: 'static,
    DecodeError: 'static,
    Build: Fn(&R, Input) -> Result<ProcessCommand, BuildError> + 'static,
    Decode: Fn(ProcessOutput) -> Result<Output, DecodeError> + 'static,
{
    type Runtime = R;
    type Input = Input;
    type Output = Output;
    type Error = AdapterError<BuildError, DecodeError>;

    fn materialise<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        input: Self::Input,
    ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
        self.adapter.execute(runtime, input)
    }
}

/// A generic `naaf_core::RepairPlanner` backed by a local process.
pub struct ProcessRepairPlanner<R, Build, Decode, Input, Artefact, Finding, BuildError, DecodeError>
{
    adapter: RepairAdapter<R, Build, Decode, Input, Artefact, Finding, BuildError, DecodeError>,
}

impl<R, Build, Decode, Input, Artefact, Finding, DecodeError>
    ProcessRepairPlanner<R, Build, Decode, Input, Artefact, Finding, Infallible, DecodeError>
{
    /// Creates a process repair planner that cannot fail while building the command.
    pub fn new(build_command: Build, decode_input: Decode) -> Self {
        Self::with_builder(build_command, decode_input)
    }
}

impl<R, Build, Decode, Input, Artefact, Finding, BuildError, DecodeError>
    ProcessRepairPlanner<R, Build, Decode, Input, Artefact, Finding, BuildError, DecodeError>
{
    /// Creates a process repair planner with an explicit build error type.
    pub fn with_builder(build_command: Build, decode_input: Decode) -> Self {
        Self {
            adapter: ProcessRoleAdapter::new(build_command, decode_input),
        }
    }
}

impl<R, Build, Decode, Input, Artefact, Finding, BuildError, DecodeError> RepairPlanner
    for ProcessRepairPlanner<R, Build, Decode, Input, Artefact, Finding, BuildError, DecodeError>
where
    R: 'static,
    Input: 'static,
    Artefact: 'static,
    Finding: 'static,
    BuildError: 'static,
    DecodeError: 'static,
    Build: Fn(&R, RepairAttempts<Input, Artefact, Finding>) -> Result<ProcessCommand, BuildError>
        + 'static,
    Decode: Fn(ProcessOutput) -> Result<Input, DecodeError> + 'static,
{
    type Runtime = R;
    type Input = Input;
    type Artefact = Artefact;
    type Finding = Finding;
    type Error = AdapterError<BuildError, DecodeError>;

    fn repair<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        attempts: Vec<Attempt<Self::Input, Self::Artefact, Self::Finding>>,
    ) -> LocalBoxFuture<'a, Result<Self::Input, Self::Error>> {
        self.adapter.execute(runtime, attempts)
    }
}

#[cfg(test)]
mod tests {
    use std::{convert::Infallible, str};

    use naaf_core::{Attempt, Check, Materialiser, RepairPlanner, Task};

    use super::{ProcessCheck, ProcessMaterialiser, ProcessRepairPlanner, ProcessTask};
    use crate::{AdapterError, ProcessAgent, ProcessCommand, ProcessError, ProcessOutput};

    #[derive(Debug, Default)]
    struct TestRuntime;

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct Workspace {
        revision: usize,
    }

    #[tokio::test]
    async fn process_task_decodes_shell_output() {
        let task = ProcessTask::new(
            |_runtime: &TestRuntime, script: String| {
                Ok::<_, Infallible>(ProcessCommand::shell(script))
            },
            |output: ProcessOutput| String::from_utf8(output.stdout),
        );

        let output = task
            .run(&TestRuntime, "printf 'hello'".to_string())
            .await
            .expect("process should succeed");

        assert_eq!(output, "hello");
    }

    #[tokio::test]
    async fn process_task_reports_exit_failures() {
        let task = ProcessTask::new(
            |_runtime: &TestRuntime, script: &'static str| {
                Ok::<_, Infallible>(ProcessCommand::shell(script))
            },
            |output: ProcessOutput| Ok::<_, Infallible>(output.stdout),
        );

        let error = task
            .run(&TestRuntime, "printf 'boom' >&2; exit 7")
            .await
            .expect_err("process should fail");

        match error {
            AdapterError::Execute(ProcessError::Exit { status, stderr, .. }) => {
                assert!(!status.success());
                assert_eq!(
                    str::from_utf8(&stderr).expect("stderr should be utf-8"),
                    "boom"
                );
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[tokio::test]
    async fn process_task_supports_direct_program_configuration() {
        let task = ProcessTask::new(
            |_runtime: &TestRuntime, _: ()| {
                Ok::<_, Infallible>(
                    ProcessCommand::new("sh")
                        .args(["-lc", "printf %s \"$PWD:$TEST_VALUE\""])
                        .current_dir(env!("CARGO_MANIFEST_DIR"))
                        .env("TEST_VALUE", "configured"),
                )
            },
            |output: ProcessOutput| String::from_utf8(output.stdout),
        );

        let output = task
            .run(&TestRuntime, ())
            .await
            .expect("process should succeed");

        assert_eq!(output, format!("{}:configured", env!("CARGO_MANIFEST_DIR")));
    }

    #[tokio::test]
    async fn process_agent_projects_task_and_check() {
        let agent = ProcessAgent::new();
        let task = agent.task(
            |_runtime: &TestRuntime, script: String| {
                Ok::<_, Infallible>(ProcessCommand::shell(script))
            },
            |output: ProcessOutput| String::from_utf8(output.stdout),
        );
        let check = agent.check(
            |_runtime: &TestRuntime, subject: String| {
                Ok::<_, Infallible>(
                    ProcessCommand::new("sh")
                        .args([
                            "-lc",
                            "if [ \"$SUBJECT\" = \"plan ready\" ]; then exit 0; fi; printf 'missing plan'",
                        ])
                        .env("SUBJECT", subject),
                )
            },
            |output: ProcessOutput| {
                let stdout = String::from_utf8(output.stdout)?;
                if stdout.is_empty() {
                    Ok::<_, std::string::FromUtf8Error>(Vec::new())
                } else {
                    Ok(vec![stdout])
                }
            },
        );

        let output = task
            .run(&TestRuntime, "printf 'plan ready'".to_string())
            .await
            .expect("task should succeed");
        let findings = check
            .check(&TestRuntime, output.clone())
            .await
            .expect("check should succeed");

        assert_eq!(output, "plan ready");
        assert!(findings.is_empty());
    }

    #[tokio::test]
    async fn process_materialiser_decodes_structured_output() {
        let materialiser = ProcessMaterialiser::new(
            |_runtime: &TestRuntime, revision: usize| {
                Ok::<_, Infallible>(ProcessCommand::shell(format!(
                    "printf 'revision={revision}'"
                )))
            },
            |output: ProcessOutput| {
                let stdout = String::from_utf8(output.stdout)?;
                let revision = stdout
                    .strip_prefix("revision=")
                    .expect("output should include prefix")
                    .parse()
                    .expect("revision should parse");
                Ok::<_, std::string::FromUtf8Error>(Workspace { revision })
            },
        );

        let output = materialiser
            .materialise(&TestRuntime, 2)
            .await
            .expect("materialiser should succeed");

        assert_eq!(output, Workspace { revision: 2 });
    }

    #[tokio::test]
    async fn process_repair_planner_decodes_next_input() {
        let planner = ProcessRepairPlanner::new(
            |_runtime: &TestRuntime, attempts: Vec<Attempt<usize, usize, &'static str>>| {
                let previous = attempts.last().expect("attempt present");
                Ok::<_, Infallible>(ProcessCommand::shell(format!(
                    "printf '{}'",
                    previous.artefact + 1
                )))
            },
            |output: ProcessOutput| {
                String::from_utf8(output.stdout)
                    .map(|stdout| stdout.parse().expect("next input should parse"))
            },
        );

        let next = planner
            .repair(
                &TestRuntime,
                vec![Attempt {
                    input: 1,
                    artefact: 2,
                    findings: vec!["tests failed"],
                }],
            )
            .await
            .expect("repair planner should succeed");

        assert_eq!(next, 3);
    }

    #[tokio::test]
    async fn process_check_supports_direct_construction() {
        let check = ProcessCheck::new(
            |_runtime: &TestRuntime, subject: String| {
                Ok::<_, Infallible>(
                    ProcessCommand::new("sh")
                        .args([
                            "-lc",
                            "if [ \"$SUBJECT\" = \"ok\" ]; then exit 0; fi; printf 'missing tests'",
                        ])
                        .env("SUBJECT", subject),
                )
            },
            |output: ProcessOutput| {
                let stdout = String::from_utf8(output.stdout)?;
                if stdout.is_empty() {
                    Ok::<_, std::string::FromUtf8Error>(Vec::new())
                } else {
                    Ok(vec![stdout])
                }
            },
        );

        let findings = check
            .check(&TestRuntime, "review this patch".to_string())
            .await
            .expect("check should succeed");

        assert_eq!(findings, vec!["missing tests".to_string()]);
    }
}
