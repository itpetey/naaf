use std::{convert::Infallible, sync::Arc};

use crate::{
    executor::Executor,
    task::{LlmCheck, LlmMaterialiser, LlmRepairPlanner, LlmTask},
};

/// Shared LLM backend that can be projected into `naaf_core` roles.
#[derive(Clone)]
pub struct LlmAgent<C, R, E = Infallible> {
    executor: Arc<Executor<C, R, E>>,
}

impl<C, R> LlmAgent<C, R, Infallible> {
    /// Creates an LLM agent without tools.
    pub fn new(client: C) -> Self {
        Self::with_executor(Executor::new(client))
    }
}

impl<C, R, E> LlmAgent<C, R, E> {
    /// Creates an LLM agent with a preconfigured executor.
    pub fn with_executor(executor: Executor<C, R, E>) -> Self {
        Self {
            executor: Arc::new(executor),
        }
    }

    /// Returns the underlying executor.
    pub fn executor(&self) -> &Executor<C, R, E> {
        self.executor.as_ref()
    }

    /// Projects this agent into a `naaf_core::Task`.
    pub fn task<Build, Decode, Input, Output, BuildError, DecodeError>(
        &self,
        build_request: Build,
        decode_output: Decode,
    ) -> LlmTask<C, R, Build, Decode, Input, Output, BuildError, DecodeError, E> {
        LlmTask::from_shared_executor(self.executor.clone(), build_request, decode_output)
    }

    /// Projects this agent into a `naaf_core::Check`.
    pub fn check<Build, Decode, Subject, Finding, BuildError, DecodeError>(
        &self,
        build_request: Build,
        decode_findings: Decode,
    ) -> LlmCheck<C, R, Build, Decode, Subject, Finding, BuildError, DecodeError, E> {
        LlmCheck::from_shared_executor(self.executor.clone(), build_request, decode_findings)
    }

    /// Projects this agent into a `naaf_core::Materialiser`.
    pub fn materialiser<Build, Decode, Input, Output, BuildError, DecodeError>(
        &self,
        build_request: Build,
        decode_output: Decode,
    ) -> LlmMaterialiser<C, R, Build, Decode, Input, Output, BuildError, DecodeError, E> {
        LlmMaterialiser::from_shared_executor(self.executor.clone(), build_request, decode_output)
    }

    /// Projects this agent into a `naaf_core::RepairPlanner`.
    pub fn repair_planner<Build, Decode, Input, Artefact, Finding, BuildError, DecodeError>(
        &self,
        build_request: Build,
        decode_input: Decode,
    ) -> LlmRepairPlanner<C, R, Build, Decode, Input, Artefact, Finding, BuildError, DecodeError, E>
    {
        LlmRepairPlanner::from_shared_executor(self.executor.clone(), build_request, decode_input)
    }
}
