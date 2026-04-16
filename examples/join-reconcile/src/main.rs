//! Demonstrates parallel fan-out with `.join()` and reconciliation with
//! `.reconcile_task()`.
//!
//! Two design tasks (`DesignApi` and `DesignUi`) run in parallel against the
//! same cloned input. Their outputs are combined by `MergeReport`.

use std::fmt::{Display, Formatter};

use futures::future::LocalBoxFuture;
use naaf_core::Task;

#[derive(Debug)]
struct PlannerRuntime;

#[derive(Clone, Debug, PartialEq, Eq)]
struct ProjectPlan {
    name: String,
    phases: Vec<String>,
    estimated_weeks: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ApiDesign {
    plan_name: String,
    endpoints: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct UiDesign {
    plan_name: String,
    components: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ProjectReport {
    plan: ProjectPlan,
    api: ApiDesign,
    ui: UiDesign,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Error;

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("planner error")
    }
}

impl std::error::Error for Error {}

struct DesignApi;

impl Task for DesignApi {
    type Runtime = PlannerRuntime;
    type Input = ProjectPlan;
    type Output = ApiDesign;
    type Error = Error;

    fn run<'a>(
        &'a self,
        _runtime: &'a Self::Runtime,
        input: Self::Input,
    ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
        Box::pin(async move {
            let endpoints = input
                .phases
                .iter()
                .map(|phase| format!("/api/{phase}"))
                .collect();
            Ok(ApiDesign {
                plan_name: input.name,
                endpoints,
            })
        })
    }
}

struct DesignUi;

impl Task for DesignUi {
    type Runtime = PlannerRuntime;
    type Input = ProjectPlan;
    type Output = UiDesign;
    type Error = Error;

    fn run<'a>(
        &'a self,
        _runtime: &'a Self::Runtime,
        input: Self::Input,
    ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
        Box::pin(async move {
            let components = input
                .phases
                .iter()
                .map(|phase| format!("{phase}Panel"))
                .collect();
            Ok(UiDesign {
                plan_name: input.name,
                components,
            })
        })
    }
}

struct MergeReport;

impl Task for MergeReport {
    type Runtime = PlannerRuntime;
    type Input = (ApiDesign, UiDesign);
    type Output = ProjectReport;
    type Error = Error;

    fn run<'a>(
        &'a self,
        _runtime: &'a Self::Runtime,
        input: Self::Input,
    ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
        Box::pin(async move {
            let plan = ProjectPlan {
                name: input.0.plan_name.clone(),
                phases: Vec::new(),
                estimated_weeks: 0,
            };
            Ok(ProjectReport {
                plan,
                api: input.0,
                ui: input.1,
            })
        })
    }
}

#[tokio::main]
async fn main() {
    let runtime = PlannerRuntime;

    let api_step = naaf_core::Step::builder(DesignApi)
        .with_findings::<()>()
        .build();
    let ui_step = naaf_core::Step::builder(DesignUi)
        .with_findings::<()>()
        .build();

    let combined = api_step.join(ui_step).reconcile_task(MergeReport);

    let plan = ProjectPlan {
        name: "Search Feature".to_string(),
        phases: vec![
            "Implement Search".to_string(),
            "Implement Indexing".to_string(),
        ],
        estimated_weeks: 3,
    };

    let result = combined
        .run(&runtime, plan)
        .await
        .expect("parallel composition should succeed");

    println!("Plan: {}", result.plan.name);
    println!("API endpoints: {:?}", result.api.endpoints);
    println!("UI components: {:?}", result.ui.components);
}
