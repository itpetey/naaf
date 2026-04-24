#![allow(clippy::type_complexity)]

use std::marker::PhantomData;

use futures::future::LocalBoxFuture;

use crate::{Attempt, Check, Materialiser, RepairPlanner, Task};

pub struct TaskFn<R, I, O, E, F> {
    f: F,
    _marker: PhantomData<fn() -> (R, I, O, E)>,
}

pub struct CheckFn<R, I, O, F, E, Fun> {
    f: Fun,
    _marker: PhantomData<fn() -> (R, I, O, F, E)>,
}

pub struct MaterialiserFn<R, I, O, E, F> {
    f: F,
    _marker: PhantomData<fn() -> (R, I, O, E)>,
}

pub struct RepairFn<R, I, A, F, E, Fun> {
    f: Fun,
    _marker: PhantomData<fn() -> (R, I, A, F, E)>,
}

pub struct RepairLastFn<R, I, A, F, E, Fun> {
    f: Fun,
    _marker: PhantomData<fn() -> (R, I, A, F, E)>,
}

impl<R, I, O, E, F> TaskFn<R, I, O, E, F> {
    pub fn new(f: F) -> Self {
        Self {
            f,
            _marker: PhantomData,
        }
    }
}

impl<R, I, O, E, F> Task for TaskFn<R, I, O, E, F>
where
    R: 'static,
    I: 'static,
    O: 'static,
    E: 'static,
    F: Fn(&R, I) -> LocalBoxFuture<'_, Result<O, E>>,
{
    type Runtime = R;
    type Input = I;
    type Output = O;
    type Error = E;

    fn run<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        input: Self::Input,
    ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
        (self.f)(runtime, input)
    }
}

impl<R, I, O, F, E, Fun> CheckFn<R, I, O, F, E, Fun> {
    pub fn new(f: Fun) -> Self {
        Self {
            f,
            _marker: PhantomData,
        }
    }
}

impl<R, I, O, F, E, Fun> Check for CheckFn<R, I, O, F, E, Fun>
where
    R: 'static,
    I: 'static,
    O: 'static,
    F: 'static,
    E: 'static,
    Fun: Fn(&R, I, O) -> LocalBoxFuture<'_, Result<Vec<F>, E>>,
{
    type Runtime = R;
    type Input = I;
    type Output = O;
    type Finding = F;
    type Error = E;

    fn check<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        input: Self::Input,
        output: Self::Output,
    ) -> LocalBoxFuture<'a, Result<Vec<Self::Finding>, Self::Error>> {
        (self.f)(runtime, input, output)
    }
}

impl<R, I, O, E, F> MaterialiserFn<R, I, O, E, F> {
    pub fn new(f: F) -> Self {
        Self {
            f,
            _marker: PhantomData,
        }
    }
}

impl<R, I, O, E, F> Materialiser for MaterialiserFn<R, I, O, E, F>
where
    R: 'static,
    I: 'static,
    O: 'static,
    E: 'static,
    F: Fn(&R, I) -> LocalBoxFuture<'_, Result<O, E>>,
{
    type Runtime = R;
    type Input = I;
    type Output = O;
    type Error = E;

    fn materialise<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        input: Self::Input,
    ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
        (self.f)(runtime, input)
    }
}

impl<R, I, A, F, E, Fun> RepairFn<R, I, A, F, E, Fun> {
    pub fn new(f: Fun) -> Self {
        Self {
            f,
            _marker: PhantomData,
        }
    }
}

impl<R, I, A, F, E, Fun> RepairPlanner for RepairFn<R, I, A, F, E, Fun>
where
    R: 'static,
    I: 'static,
    A: 'static,
    F: 'static,
    E: 'static,
    Fun: Fn(&R, Vec<Attempt<I, A, F>>) -> LocalBoxFuture<'_, Result<I, E>>,
{
    type Runtime = R;
    type Input = I;
    type Output = A;
    type Finding = F;
    type Error = E;

    fn repair<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        attempts: Vec<Attempt<Self::Input, Self::Output, Self::Finding>>,
    ) -> LocalBoxFuture<'a, Result<Self::Input, Self::Error>> {
        (self.f)(runtime, attempts)
    }
}

impl<R, I, A, F, E, Fun> RepairLastFn<R, I, A, F, E, Fun> {
    pub fn new(f: Fun) -> Self {
        Self {
            f,
            _marker: PhantomData,
        }
    }
}

impl<R, I, A, F, E, Fun> RepairPlanner for RepairLastFn<R, I, A, F, E, Fun>
where
    R: 'static,
    I: 'static,
    A: 'static,
    F: 'static,
    E: 'static,
    Fun: Fn(&R, Attempt<I, A, F>) -> LocalBoxFuture<'_, Result<I, E>>,
{
    type Runtime = R;
    type Input = I;
    type Output = A;
    type Finding = F;
    type Error = E;

    fn repair<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        attempts: Vec<Attempt<Self::Input, Self::Output, Self::Finding>>,
    ) -> LocalBoxFuture<'a, Result<Self::Input, Self::Error>> {
        let last = attempts.into_iter().last().expect("attempt present");
        (self.f)(runtime, last)
    }
}

pub fn check_fn<R, I, O, F, E, Fun>(f: Fun) -> CheckFn<R, I, O, F, E, Fun>
where
    Fun: Fn(&R, I, O) -> LocalBoxFuture<'_, Result<Vec<F>, E>>,
{
    CheckFn::new(f)
}

pub fn materialiser_fn<R, I, O, E, F>(f: F) -> MaterialiserFn<R, I, O, E, F>
where
    F: Fn(&R, I) -> LocalBoxFuture<'_, Result<O, E>>,
{
    MaterialiserFn::new(f)
}

pub fn repair_fn<R, I, A, F, E, Fun>(f: Fun) -> RepairFn<R, I, A, F, E, Fun>
where
    Fun: Fn(&R, Vec<Attempt<I, A, F>>) -> LocalBoxFuture<'_, Result<I, E>>,
{
    RepairFn::new(f)
}

pub fn repair_last_fn<R, I, A, F, E, Fun>(f: Fun) -> RepairLastFn<R, I, A, F, E, Fun>
where
    Fun: Fn(&R, Attempt<I, A, F>) -> LocalBoxFuture<'_, Result<I, E>>,
{
    RepairLastFn::new(f)
}

pub fn task_fn<R, I, O, E, F>(f: F) -> TaskFn<R, I, O, E, F>
where
    F: Fn(&R, I) -> LocalBoxFuture<'_, Result<O, E>>,
{
    TaskFn::new(f)
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use crate::{Check, Materialiser, RepairPlanner, RetryPolicy, Step, Task};

    #[derive(Debug)]
    struct Runtime;

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct Input {
        value: usize,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct Output {
        value: usize,
    }

    #[tokio::test]
    async fn task_fn_implements_task() {
        let task = super::task_fn(|_rt: &Runtime, input: Input| {
            Box::pin(async move {
                Ok::<_, Infallible>(Output {
                    value: input.value * 2,
                })
            })
        });

        let output: Result<Output, Infallible> = task.run(&Runtime, Input { value: 3 }).await;
        assert_eq!(output.expect("task should succeed").value, 6);
    }

    #[tokio::test]
    async fn check_fn_implements_check() {
        let check = super::check_fn(|_rt: &Runtime, _input: Input, output: Output| {
            let findings = if output.value < 10 {
                vec!["too small"]
            } else {
                Vec::new()
            };
            Box::pin(async move { Ok::<_, Infallible>(findings) })
        });

        let findings: Result<Vec<&'static str>, Infallible> = check
            .check(&Runtime, Input { value: 0 }, Output { value: 5 })
            .await;
        assert_eq!(findings.expect("check should succeed"), vec!["too small"]);

        let findings: Result<Vec<&'static str>, Infallible> = check
            .check(&Runtime, Input { value: 0 }, Output { value: 15 })
            .await;
        assert!(findings.expect("check should succeed").is_empty());
    }

    #[tokio::test]
    async fn materialiser_fn_implements_materialiser() {
        let mat = super::materialiser_fn(|_rt: &Runtime, input: Output| {
            Box::pin(async move { Ok::<_, Infallible>(format!("output={}", input.value)) })
        });

        let result: Result<String, Infallible> =
            mat.materialise(&Runtime, Output { value: 42 }).await;
        assert_eq!(result.expect("materialiser should succeed"), "output=42");
    }

    #[tokio::test]
    async fn repair_last_fn_implements_repair_planner() {
        use crate::Attempt;

        let planner = super::repair_last_fn(
            |_rt: &Runtime, last: Attempt<Input, Output, &'static str>| {
                Box::pin(async move {
                    Ok::<_, Infallible>(Input {
                        value: last.output.value + 1,
                    })
                })
            },
        );

        let attempts = vec![Attempt {
            input: Input { value: 3 },
            output: Output { value: 6 },
            findings: vec!["too small"],
        }];

        let result: Result<Input, Infallible> = planner.repair(&Runtime, attempts).await;
        assert_eq!(result.expect("repair should succeed").value, 7);
    }

    #[tokio::test]
    async fn step_task_builds_closure_step_without_findings() {
        let step = Step::task(|_rt: &Runtime, input: Input| {
            Box::pin(async move {
                Ok::<_, Infallible>(Output {
                    value: input.value + 1,
                })
            })
        });

        let result = step.run(&Runtime, Input { value: 9 }).await;
        assert_eq!(result.expect("step should succeed").value, 10);
    }

    #[tokio::test]
    async fn closure_step_with_validation_and_repair() {
        use crate::{Attempt, check_fn, repair_last_fn, task_fn};

        let task = task_fn(|_rt: &Runtime, input: Input| {
            Box::pin(async move { Ok::<_, Infallible>(Output { value: input.value }) })
        });

        let check = check_fn(|_rt: &Runtime, _input: Input, output: Output| {
            let findings: Vec<&'static str> = if output.value < 3 {
                vec!["too low"]
            } else {
                Vec::new()
            };
            Box::pin(async move { Ok::<_, Infallible>(findings) })
        });

        let repair = repair_last_fn(
            |_rt: &Runtime, last: Attempt<Input, Output, &'static str>| {
                Box::pin(async move {
                    Ok::<_, Infallible>(Input {
                        value: last.input.value + 2,
                    })
                })
            },
        );

        let step = Step::builder(task)
            .validate(check)
            .repair_with(repair)
            .retry_policy(RetryPolicy::new(5))
            .build();

        let traced = step.run_traced(&Runtime, Input { value: 0 }).await;
        let traced = traced.expect("step should succeed");
        assert_eq!(traced.output().value, 4);
        assert_eq!(traced.report().attempt_count(), 3);
    }
}
