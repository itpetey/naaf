use naaf_core::context::RunContext;
use naaf_schema::artifact::ArtifactRef;

use crate::executor::Executor;

pub struct LlmExecutor;

impl Executor for LlmExecutor {
    fn id(&self) -> &'static str {
        "llm"
    }

    fn execute(
        &self,
        _ctx: &mut RunContext,
        _inputs: Vec<ArtifactRef>,
    ) -> Result<ArtifactRef, String> {
        // stub
        Err("not implemented".into())
    }
}
