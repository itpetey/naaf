use crate::task::{ProcessCheck, ProcessMaterialiser, ProcessRepairPlanner, ProcessTask};

/// Shared process backend that can be projected into `naaf_core` roles.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProcessAgent;

impl ProcessAgent {
    /// Creates a process agent.
    pub const fn new() -> Self {
        Self
    }

    /// Projects this agent into a `naaf_core::Task`.
    pub fn task<R, Build, Decode, Input, Output, BuildError, DecodeError>(
        &self,
        build_command: Build,
        decode_output: Decode,
    ) -> ProcessTask<R, Build, Decode, Input, Output, BuildError, DecodeError> {
        ProcessTask::with_builder(build_command, decode_output)
    }

    /// Projects this agent into a `naaf_core::Check`.
    pub fn check<R, Build, Decode, Input, Output, Finding, BuildError, DecodeError>(
        &self,
        build_command: Build,
        decode_findings: Decode,
    ) -> ProcessCheck<R, Build, Decode, Input, Output, Finding, BuildError, DecodeError> {
        ProcessCheck::with_builder(build_command, decode_findings)
    }

    /// Projects this agent into a `naaf_core::Materialiser`.
    pub fn materialiser<R, Build, Decode, Input, Output, BuildError, DecodeError>(
        &self,
        build_command: Build,
        decode_output: Decode,
    ) -> ProcessMaterialiser<R, Build, Decode, Input, Output, BuildError, DecodeError> {
        ProcessMaterialiser::with_builder(build_command, decode_output)
    }

    /// Projects this agent into a `naaf_core::RepairPlanner`.
    pub fn repair_planner<R, Build, Decode, Input, Output, Finding, BuildError, DecodeError>(
        &self,
        build_command: Build,
        decode_input: Decode,
    ) -> ProcessRepairPlanner<R, Build, Decode, Input, Output, Finding, BuildError, DecodeError>
    {
        ProcessRepairPlanner::with_builder(build_command, decode_input)
    }
}
