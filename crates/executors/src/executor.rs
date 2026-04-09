use naaf_core::context::RunContext;
use naaf_schema::artifact::ArtifactRef;

pub trait Executor {
    fn id(&self) -> &'static str;

    fn execute(
        &self,
        ctx: &mut RunContext,
        inputs: Vec<ArtifactRef>,
    ) -> Result<ArtifactRef, String>;
}
