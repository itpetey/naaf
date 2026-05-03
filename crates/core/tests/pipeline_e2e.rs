use std::sync::{Arc, Mutex};

use futures::future::LocalBoxFuture;
use naaf_core::{Phase, PhaseId, Pipeline, PipelineCheckpoint, PipelineCheckpointer, Route, Step};

#[derive(Debug)]
struct Runtime;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Error;

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("pipeline test error")
    }
}

impl std::error::Error for Error {}

#[derive(Clone)]
struct Merge;

impl Phase for Merge {
    type Runtime = Runtime;
    type Input = Vec<usize>;
    type Output = usize;
    type Error = Error;

    fn run<'a>(
        &'a self,
        _runtime: &'a Runtime,
        input: Vec<usize>,
    ) -> LocalBoxFuture<'a, Result<usize, Error>> {
        Box::pin(async move { Ok(input.into_iter().sum()) })
    }
}

#[derive(Clone, Default)]
struct MemoryCheckpointer {
    checkpoint: Arc<Mutex<Option<PipelineCheckpoint>>>,
}

impl PipelineCheckpointer for MemoryCheckpointer {
    fn save_pipeline(
        &self,
        checkpoint: PipelineCheckpoint,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = naaf_core::CheckpointResult<()>> + Send>>
    {
        *self.checkpoint.lock().expect("checkpoint lock") = Some(checkpoint);
        Box::pin(async { Ok(()) })
    }

    fn load_pipeline(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = naaf_core::CheckpointResult<Option<PipelineCheckpoint>>,
                > + Send,
        >,
    > {
        let checkpoint = self.checkpoint.lock().expect("checkpoint lock").clone();
        Box::pin(async move { Ok(checkpoint) })
    }
}

#[tokio::test]
async fn pipeline_runs_step_wrapped_parallel_join_phase() {
    let add_one = Step::task(|_runtime: &Runtime, input: usize| {
        Box::pin(async move { Ok::<_, Error>(input + 1) })
    });
    let double = Step::task(|_runtime: &Runtime, input: usize| {
        Box::pin(async move { Ok::<_, Error>(input * 2) })
    });

    let pipeline = Pipeline::builder()
        .add_step(PhaseId::new("start"), add_one)
        .add_step(PhaseId::new("a"), double.clone())
        .add_step(PhaseId::new("b"), double)
        .add_phase(PhaseId::new("merge"), Merge)
        .with_route(PhaseId::new("start"), Route::parallel(["a", "b"]))
        .with_route(PhaseId::new("a"), Route::Halt)
        .with_route(PhaseId::new("b"), Route::Halt)
        .with_route(PhaseId::new("merge"), Route::Halt)
        .with_parallel_join("start", "merge")
        .with_initial(PhaseId::new("start"))
        .build()
        .expect("pipeline should build");

    let output: usize = pipeline.run(&Runtime, 3usize).await.expect("pipeline runs");

    assert_eq!(output, 16);
}

#[tokio::test]
async fn pipeline_checkpointer_loads_and_resumes_round_trip() {
    let checkpointer = MemoryCheckpointer::default();
    let handle = checkpointer.checkpoint.clone();

    let pipeline = Pipeline::builder()
        .add_persistent_step(
            PhaseId::new("add"),
            Step::task(|_runtime: &Runtime, input: usize| {
                Box::pin(async move { Ok::<_, Error>(input + 1) })
            }),
        )
        .add_persistent_step(
            PhaseId::new("double"),
            Step::task(|_runtime: &Runtime, input: usize| {
                Box::pin(async move { Ok::<_, Error>(input * 2) })
            }),
        )
        .with_route(PhaseId::new("add"), Route::next("double"))
        .with_route(PhaseId::new("double"), Route::Halt)
        .with_initial(PhaseId::new("add"))
        .checkpoint_with(checkpointer)
        .build()
        .expect("pipeline should build");

    let output: usize = pipeline.run(&Runtime, 4usize).await.expect("pipeline runs");
    assert_eq!(output, 10);

    let saved = handle.lock().expect("checkpoint lock").clone();
    assert!(saved.is_some());

    let resumed: Option<usize> = pipeline.resume(&Runtime).await.expect("resume succeeds");
    assert_eq!(resumed, Some(10));
}
